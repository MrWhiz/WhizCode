use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;
use walkdir::WalkDir;
use rusqlite::{params, Connection, Result as SqliteResult};
use crate::error::Result;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: String,
    pub file_path: String,
    pub chunk_type: String, // function, class, interface, etc.
    pub symbol_name: Option<String>,
    pub content: String,
    pub start_line: u32,
    pub end_line: u32,
    pub embedding: Vec<f32>,
    pub complexity: u32,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: CodeChunk,
    pub similarity_score: f32,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticQuery {
    pub query: String,
    pub file_path: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_chunks: usize,
    pub total_files: usize,
    pub index_size_bytes: usize,
    pub last_updated: i64,
}

pub struct VectorSearchSystem {
    pub db: Arc<Mutex<Connection>>,
    #[allow(dead_code)]
    index_dir: PathBuf,
}

impl VectorSearchSystem {
    pub fn new(workspace_root: &str) -> crate::error::Result<Self> {
        let index_dir = Path::new(workspace_root)
            .join(".whizcode")
            .join("vector-index");

        fs::create_dir_all(&index_dir)
            .map_err(|e| format!("Failed to create index directory: {}", e))?;

        let db_path = index_dir.join("vector_store.db");
        let conn = Connection::open(db_path)
            .map_err(|e| format!("Failed to open vector database: {}", e))?;

        // Initialize schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS code_chunks (
                id TEXT PRIMARY KEY,
                file_path TEXT,
                chunk_type TEXT,
                symbol_name TEXT,
                content TEXT,
                start_line INTEGER,
                end_line INTEGER,
                embedding TEXT,
                complexity INTEGER,
                dependencies TEXT
            )",
            [],
        ).map_err(|e| format!("Failed to create schema: {}", e))?;

        Ok(VectorSearchSystem {
            db: Arc::new(Mutex::new(conn)),
            index_dir,
        })
    }

    pub fn index_workspace(&self, workspace_path: &str) -> crate::error::Result<()> {
        let conn = self.db.lock().unwrap();
        
        // Clear existing index for this workspace (or just keep it and sync - for simplicity we clear now)
        conn.execute("DELETE FROM code_chunks", []).map_err(|e| format!("Failed to clear index: {}", e))?;

        let mut file_count = 0;
        let mut chunk_count = 0;

        for entry in WalkDir::new(workspace_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            if crate::utils::should_skip_file(path) {
                continue;
            }

            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go") {
                if let Ok(content) = fs::read_to_string(path) {
                    let file_path = path.to_string_lossy().to_string();
                    let file_chunks = self.chunk_file(&content, &file_path);
                    
                    for chunk in file_chunks {
                        let embedding_json = serde_json::to_string(&chunk.embedding).unwrap_or_default();
                        let deps_json = serde_json::to_string(&chunk.dependencies).unwrap_or_default();
                        
                        conn.execute(
                            "INSERT INTO code_chunks (id, file_path, chunk_type, symbol_name, content, start_line, end_line, embedding, complexity, dependencies)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                            params![
                                chunk.id,
                                chunk.file_path,
                                chunk.chunk_type,
                                chunk.symbol_name,
                                chunk.content,
                                chunk.start_line,
                                chunk.end_line,
                                embedding_json,
                                chunk.complexity,
                                deps_json
                            ],
                        ).map_err(|e| format!("Failed to insert chunk: {}", e))?;
                        chunk_count += 1;
                    }
                    file_count += 1;
                }
            }
        }

        eprintln!("[VECTOR] Indexed {} files, {} chunks", file_count, chunk_count);
        Ok(())
    }

    pub fn semantic_search(&self, query: &SemanticQuery) -> crate::error::Result<Vec<SearchResult>> {
        let query_embedding = self.generate_embedding(&query.query);
        let conn = self.db.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, file_path, chunk_type, symbol_name, content, start_line, end_line, embedding, complexity, dependencies FROM code_chunks"
        ).map_err(|e| format!("Failed to prepare search: {}", e))?;

        let chunk_iter = stmt.query_map([], |row| {
            let embedding_str: String = row.get(7)?;
            let deps_str: String = row.get(9)?;
            
            let embedding: Vec<f32> = serde_json::from_str(&embedding_str).unwrap_or_default();
            let dependencies: Vec<String> = serde_json::from_str(&deps_str).unwrap_or_default();

            Ok(CodeChunk {
                id: row.get(0)?,
                file_path: row.get(1)?,
                chunk_type: row.get(2)?,
                symbol_name: row.get(3)?,
                content: row.get(4)?,
                start_line: row.get(5)?,
                end_line: row.get(6)?,
                embedding,
                complexity: row.get(8)?,
                dependencies,
            })
        }).map_err(|e| format!("Query failed: {}", e))?;

        let mut results = Vec::new();
        for chunk_res in chunk_iter {
            if let Ok(chunk) = chunk_res {
                let similarity = self.cosine_similarity(&query_embedding, &chunk.embedding);
                if similarity > 0.3 { // Threshold
                    results.push(SearchResult {
                        chunk,
                        similarity_score: similarity,
                        relevance_score: similarity,
                    });
                }
            }
        }

        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        // Synchronous fallback: simple TF-style hash embedding (used when Ollama is unavailable)
        let mut embedding = vec![0.0f32; 64];
        let words: Vec<&str> = text.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            let h = word.bytes().fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
            embedding[h % 64] += 1.0 / (1.0 + i as f32 * 0.1);
        }
        let norm = (embedding.iter().map(|x| x * x).sum::<f32>()).sqrt();
        if norm > 0.0 {
            for x in embedding.iter_mut() { *x /= norm; }
        }
        embedding
    }

    pub async fn generate_embedding_ollama(text: &str, model: &str) -> Vec<f32> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({ "model": model, "prompt": text });
        if let Ok(resp) = client
            .post("http://localhost:11434/api/embeddings")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = json.get("embedding").and_then(|e| e.as_array()) {
                    let v: Vec<f32> = arr.iter()
                        .filter_map(|x| x.as_f64().map(|f| f as f32))
                        .collect();
                    if !v.is_empty() {
                        return v;
                    }
                }
            }
        }
        // Fallback to hash embedding if Ollama unavailable
        let mut embedding = vec![0.0f32; 64];
        let words: Vec<&str> = text.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            let h = word.bytes().fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
            embedding[h % 64] += 1.0 / (1.0 + i as f32 * 0.1);
        }
        let norm = (embedding.iter().map(|x| x * x).sum::<f32>()).sqrt();
        if norm > 0.0 {
            for x in embedding.iter_mut() { *x /= norm; }
        }
        embedding
    }

    #[allow(dead_code)]
    pub async fn index_workspace_with_embeddings(&self, workspace_path: &str, embedding_model: &str) -> crate::error::Result<()> {
        Self::index_workspace_with_embeddings_db(Arc::clone(&self.db), workspace_path, embedding_model).await
    }

    pub async fn index_workspace_with_embeddings_db(db: Arc<Mutex<Connection>>, workspace_path: &str, embedding_model: &str) -> crate::error::Result<()> {
        {
            let conn = db.lock().unwrap();
            conn.execute("DELETE FROM code_chunks", []).map_err(|e| format!("Failed to clear index: {}", e))?;
        }

        let mut file_count = 0;
        let mut chunk_count = 0;

        // Collect all chunks first (sync), then embed + insert (async)
        let mut pending: Vec<(String, String, String, u32, u32)> = Vec::new(); // (id, file_path, content, start, end)

        for entry in WalkDir::new(workspace_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            if crate::utils::should_skip_file(path) { continue; }

            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "md" | "json" | "toml") {
                if let Ok(content) = fs::read_to_string(path) {
                    let file_path = path.to_string_lossy().to_string();
                    let lines: Vec<&str> = content.lines().collect();
                    for (i, window) in lines.chunks(40).enumerate() {
                        let start = (i * 40) as u32;
                        let end = start + window.len() as u32;
                        let chunk_content = window.join("\n");
                        let id = format!("{}:{}", file_path, start);
                        pending.push((id, file_path.clone(), chunk_content, start, end));
                    }
                    file_count += 1;
                }
            }
        }

        for (id, file_path, chunk_content, start, end) in &pending {
            // Async embedding - no lock held
            let embedding = Self::generate_embedding_ollama(chunk_content, embedding_model).await;
            let embedding_json = serde_json::to_string(&embedding).unwrap_or_default();

            // Lock only for the insert
            let conn = db.lock().unwrap();
            let _ = conn.execute(
                "INSERT OR REPLACE INTO code_chunks (id, file_path, chunk_type, symbol_name, content, start_line, end_line, embedding, complexity, dependencies)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![id, file_path, "block", Option::<String>::None, chunk_content,
                        start, end, embedding_json, 1u32, "[]"],
            );
            chunk_count += 1;
        }

        eprintln!("[VECTOR] Indexed {} files, {} chunks with Ollama embeddings", file_count, chunk_count);
        Ok(())
    }

    /// Build a compact file tree string for injection into the system prompt
    pub fn build_file_tree(workspace_path: &str, max_files: usize) -> String {
        let mut lines = vec![format!("workspace: {}", workspace_path)];
        let mut count = 0;

        for entry in WalkDir::new(workspace_path)
            .max_depth(4)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if count >= max_files { break; }
            let path = entry.path();
            if crate::utils::should_skip_file(path) { continue; }

            let depth = entry.depth();
            let indent = "  ".repeat(depth);
            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if path.is_dir() {
                lines.push(format!("{}📁 {}/", indent, name));
            } else {
                lines.push(format!("{}📄 {}", indent, name));
                count += 1;
            }
        }

        lines.join("\n")
    }

    fn cosine_similarity(&self, v1: &[f32], v2: &[f32]) -> f32 {
        if v1.len() != v2.len() || v1.is_empty() {
            return 0.0;
        }
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        dot_product // Assuming normalized vectors
    }

    fn chunk_file(&self, content: &str, file_path: &str) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        // Very basic chunking by blocks of 30 lines
        for (i, window) in lines.chunks(30).enumerate() {
            let start = i * 30;
            let end = start + window.len();
            let chunk_content = window.join("\n");
            let id = format!("{}:{}", file_path, start);
            
            chunks.push(CodeChunk {
                id,
                file_path: file_path.to_string(),
                chunk_type: "block".to_string(),
                symbol_name: None,
                content: chunk_content.clone(),
                start_line: (start + 1) as u32,
                end_line: end as u32,
                embedding: self.generate_embedding(&chunk_content),
                complexity: 1,
                dependencies: vec![],
            });
        }
        chunks
    }

    pub fn get_index_stats(&self) -> SqliteResult<IndexStats> {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*), COUNT(DISTINCT file_path) FROM code_chunks")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(IndexStats {
                total_chunks: row.get(0)?,
                total_files: row.get(1)?,
                index_size_bytes: 0, // Hard to estimate exactly
                last_updated: chrono::Utc::now().timestamp(),
            })
        } else {
            Ok(IndexStats {
                total_chunks: 0,
                total_files: 0,
                index_size_bytes: 0,
                last_updated: 0,
            })
        }
    }
}

// Commands
#[tauri::command]
pub async fn vector_index_workspace(
    workspace_path: String,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<()> {
    let system = state.lock().unwrap();
    system.index_workspace(&workspace_path)
}

#[tauri::command]
pub async fn vector_index_workspace_full(
    workspace_path: String,
    embedding_model: Option<String>,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<()> {
    let model = embedding_model.unwrap_or_else(|| "nomic-embed-text".to_string());
    // Extract the db Arc without holding the outer MutexGuard across an await
    let db = {
        let system = state.lock().unwrap();
        Arc::clone(&system.db)
    };
    VectorSearchSystem::index_workspace_with_embeddings_db(db, &workspace_path, &model).await
}

#[tauri::command]
pub async fn vector_get_file_tree(
    workspace_path: String,
    max_files: Option<usize>,
) -> Result<String> {
    Ok(VectorSearchSystem::build_file_tree(&workspace_path, max_files.unwrap_or(300)))
}

#[tauri::command]
pub async fn vector_semantic_search(
    query: SemanticQuery,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<SearchResult>> {
    let system = state.lock().unwrap();
    system.semantic_search(&query)
}

#[tauri::command]
pub async fn vector_get_index_stats(
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<IndexStats> {
    let system = state.lock().unwrap();
    system.get_index_stats().map_err(|e| format!("DB failed: {}", e).into())
}

#[tauri::command]
pub async fn vector_find_similar(
    _query: String,
    _state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<SearchResult>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn vector_get_recommendations(
    _state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<String>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn vector_update_file(
    _path: String,
    _state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<()> {
    Ok(())
}

#[tauri::command]
pub async fn vector_get_stats(
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<IndexStats> {
    let system = state.lock().unwrap();
    system.get_index_stats().map_err(|e| format!("DB failed: {}", e).into())
}

#[tauri::command]
pub async fn vector_clear_index(
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<()> {
    let system = state.lock().unwrap();
    let conn = system.db.lock().unwrap();
    conn.execute("DELETE FROM code_chunks", [])
        .map_err(|e| crate::error::ApiError {
            code: "DB_ERROR".to_string(),
            message: format!("Clear failed: {}", e),
            details: None,
        })?;
    Ok(())
}
