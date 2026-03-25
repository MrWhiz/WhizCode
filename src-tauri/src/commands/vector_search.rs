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
        
        // Ensure index table has mtime column for incremental indexing
        // This will fail if the column already exists, which is fine.
        let _ = conn.execute("ALTER TABLE code_chunks ADD COLUMN mtime INTEGER", []);

        let mut file_count = 0;
        let mut chunk_count = 0;

        for entry in WalkDir::new(workspace_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            let file_path = path.to_string_lossy().to_string();
            
            // Skip hidden or ignored files
            if crate::utils::should_skip_file(path) || file_path.contains(".git") || file_path.contains("node_modules") {
                continue;
            }

            let mtime = fs::metadata(path)
                .and_then(|m| m.modified().map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()))
                .unwrap_or(0);

            // Check if file has changed since last index
            let mut stmt = conn.prepare("SELECT mtime FROM code_chunks WHERE file_path = ? LIMIT 1").unwrap();
            let last_mtime: Option<u64> = stmt.query_row(params![file_path], |row| row.get(0)).ok();
            
            if let Some(last) = last_mtime {
                if last >= mtime {
                    continue; // Skip unchanged file
                }
                // File changed, remove old chunks before re-indexing
                let _ = conn.execute("DELETE FROM code_chunks WHERE file_path = ?", params![file_path]);
            }

            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go") {
                if let Ok(content) = fs::read_to_string(path) {
                    let file_chunks = self.chunk_file(&content, &file_path);
                    
                    for chunk in file_chunks {
                        let embedding_json = serde_json::to_string(&chunk.embedding).unwrap_or_default();
                        let deps_json = serde_json::to_string(&chunk.dependencies).unwrap_or_default();
                        
                        conn.execute(
                            "INSERT INTO code_chunks (id, file_path, chunk_type, symbol_name, content, start_line, end_line, embedding, complexity, dependencies, mtime)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                            params![
                                chunk.id,
                                chunk.file_path,
                                chunk.chunk_type,
                                chunk.symbol_name,
                                chunk.content,
                                chunk.start_line,
                                chunk.end_line,
                                embedding_json,
                                chunk.complexity as i32,
                                deps_json,
                                mtime as i64,
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
                if let Some(file_path) = &query.file_path {
                    if !chunk.file_path.contains(file_path) {
                        continue;
                    }
                }
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

    fn index_single_file(&self, path: &Path) -> crate::error::Result<()> {
        if crate::utils::should_skip_file(path) || !path.is_file() {
            return Ok(());
        }

        let file_path = path.to_string_lossy().to_string();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "md" | "json" | "toml") {
            return Ok(());
        }

        let conn = self.db.lock().unwrap();
        conn.execute("DELETE FROM code_chunks WHERE file_path = ?", params![file_path.clone()])
            .map_err(|e| format!("Failed to clear file chunks: {}", e))?;

        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file for indexing: {}", e))?;
        let chunks = self.chunk_file(&content, &file_path);
        let mtime = fs::metadata(path)
            .and_then(|m| m.modified().map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()))
            .unwrap_or(0) as i64;

        for chunk in chunks {
            let embedding_json = serde_json::to_string(&chunk.embedding).unwrap_or_default();
            let deps_json = serde_json::to_string(&chunk.dependencies).unwrap_or_default();
            conn.execute(
                "INSERT INTO code_chunks (id, file_path, chunk_type, symbol_name, content, start_line, end_line, embedding, complexity, dependencies, mtime)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    chunk.id,
                    chunk.file_path,
                    chunk.chunk_type,
                    chunk.symbol_name,
                    chunk.content,
                    chunk.start_line,
                    chunk.end_line,
                    embedding_json,
                    chunk.complexity as i32,
                    deps_json,
                    mtime,
                ],
            ).map_err(|e| format!("Failed to insert file chunk: {}", e))?;
        }

        Ok(())
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

        // Regex-free symbol detection (faster for local indexing)
        let mut current_chunk = Vec::new();
        let mut chunk_start = 0;
        let mut current_symbol = None;
        let mut current_type = "block".to_string();

        let symbols = ["function", "class", "async", "interface", "struct", "enum", "def", "trait", "impl", "pub fn", "fn", "type"];

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let is_symbol_start = symbols.iter().any(|s| trimmed.contains(s)) && (trimmed.contains('{') || trimmed.contains(':'));

            if is_symbol_start && !current_chunk.is_empty() {
                // Save previous chunk
                let content = current_chunk.join("\n");
                chunks.push(self.create_chunk(file_path, &content, chunk_start + 1, i as u32, current_symbol.clone(), &current_type));
                current_chunk.clear();
                chunk_start = i as u32;

                // Identify new symbol name
                for s in symbols {
                    if trimmed.contains(s) {
                        current_type = s.to_string();
                        // Extract approximate name
                        let parts: Vec<&str> = trimmed.split(|c| " (:{<".contains(c)).collect();
                        if let Some(name) = parts.iter().skip_while(|p| !p.contains(s)).nth(1) {
                            current_symbol = Some(name.to_string());
                        }
                        break;
                    }
                }
            }

            current_chunk.push(*line);

            // Cap chunk size at ~100 lines to prevent context bloat
            if current_chunk.len() > 100 {
                let content = current_chunk.join("\n");
                chunks.push(self.create_chunk(file_path, &content, chunk_start + 1, (i + 1) as u32, current_symbol.clone(), &current_type));
                current_chunk.clear();
                chunk_start = (i + 1) as u32;
                current_symbol = None;
                current_type = "block".to_string();
            }
        }

        // Last chunk
        if !current_chunk.is_empty() {
            let content = current_chunk.join("\n");
            chunks.push(self.create_chunk(file_path, &content, chunk_start + 1, lines.len() as u32, current_symbol, &current_type));
        }

        chunks
    }

    fn create_chunk(&self, file_path: &str, content: &str, start: u32, end: u32, symbol: Option<String>, chunk_type: &str) -> CodeChunk {
        // ENHANCEMENT: Prepend context to content for better embedding relevance
        let enhanced_content = if let Some(ref s) = symbol {
            format!("File: {} | {} {}: {}\n{}", file_path, chunk_type, s, s, content)
        } else {
            format!("File: {} | Block:\n{}", file_path, content)
        };

        CodeChunk {
            id: format!("{}:{}", file_path, start),
            file_path: file_path.to_string(),
            chunk_type: chunk_type.to_string(),
            symbol_name: symbol,
            content: content.to_string(),
            start_line: start,
            end_line: end,
            embedding: self.generate_embedding(&enhanced_content),
            complexity: 1,
            dependencies: vec![],
        }
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
    query: String,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<SearchResult>> {
    let system = state.lock().unwrap();
    system.semantic_search(&SemanticQuery {
        query,
        file_path: None,
        limit: Some(5),
    })
}

#[tauri::command]
pub async fn vector_get_recommendations(
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<String>> {
    let system = state.lock().unwrap();
    let stats = system.get_index_stats().map_err(|e| format!("DB failed: {}", e))?;
    let mut recommendations = Vec::new();
    if stats.total_chunks == 0 {
        recommendations.push("Vector index is empty; run a full workspace index before semantic search.".to_string());
    }
    if stats.total_files < 25 {
        recommendations.push("Workspace grounding is shallow; consider a full embedding index for better semantic recall.".to_string());
    }
    if recommendations.is_empty() {
        recommendations.push("Vector search index is healthy.".to_string());
    }
    Ok(recommendations)
}

#[tauri::command]
pub async fn vector_update_file(
    path: String,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<()> {
    let system = state.lock().unwrap();
    system.index_single_file(Path::new(&path))
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
