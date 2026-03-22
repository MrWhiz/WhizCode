use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;
use walkdir::WalkDir;

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

#[allow(dead_code)]
pub struct VectorSearchSystem {
    chunks: Arc<Mutex<HashMap<String, CodeChunk>>>,
    index_dir: PathBuf,
    stats: Arc<Mutex<IndexStats>>,
}

impl VectorSearchSystem {
    pub fn new(workspace_path: &str) -> Result<Self, String> {
        let index_dir = Path::new(workspace_path)
            .join(".whizcode")
            .join("vector-index");

        fs::create_dir_all(&index_dir)
            .map_err(|e| format!("Failed to create index directory: {}", e))?;

        Ok(VectorSearchSystem {
            chunks: Arc::new(Mutex::new(HashMap::new())),
            index_dir,
            stats: Arc::new(Mutex::new(IndexStats {
                total_chunks: 0,
                total_files: 0,
                index_size_bytes: 0,
                last_updated: chrono::Utc::now().timestamp(),
            })),
        })
    }

    pub fn index_workspace(&self, workspace_path: &str) -> Result<(), String> {
        let mut chunks = self.chunks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        chunks.clear();
        let mut file_count = 0;

        // Find all code files
        for entry in WalkDir::new(workspace_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            let extension = path.extension().and_then(|s| s.to_str());

            // Only index code files
            if matches!(
                extension,
                Some("rs") | Some("ts") | Some("tsx") | Some("js") | Some("jsx")
                    | Some("py") | Some("go") | Some("java") | Some("cpp") | Some("c")
            ) {
                if let Ok(content) = fs::read_to_string(path) {
                    let file_chunks = self.chunk_file(&content, path.to_string_lossy().as_ref());
                    for chunk in file_chunks {
                        chunks.insert(chunk.id.clone(), chunk);
                    }
                    file_count += 1;
                }
            }
        }

        stats.total_chunks = chunks.len();
        stats.total_files = file_count;
        stats.index_size_bytes = Self::estimate_index_size(&chunks);
        stats.last_updated = chrono::Utc::now().timestamp();

        Ok(())
    }

    pub fn semantic_search(&self, query: &SemanticQuery) -> Result<Vec<SearchResult>, String> {
        let chunks = self.chunks.lock().unwrap();

        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // Generate embedding for query
        let query_embedding = Self::generate_embedding(&query.query);

        let mut results: Vec<SearchResult> = chunks
            .values()
            .filter(|chunk| {
                if let Some(ref file_path) = query.file_path {
                    chunk.file_path.contains(file_path)
                } else {
                    true
                }
            })
            .map(|chunk| {
                let similarity = Self::cosine_similarity(&query_embedding, &chunk.embedding);
                let relevance = Self::calculate_relevance_score(chunk, &query.query, similarity);

                SearchResult {
                    chunk: chunk.clone(),
                    similarity_score: similarity,
                    relevance_score: relevance,
                }
            })
            .collect();

        // Sort by relevance score
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        let limit = query.limit.unwrap_or(10);
        Ok(results.into_iter().take(limit).collect())
    }

    pub fn find_similar_code(&self, code_snippet: &str) -> Result<Vec<SearchResult>, String> {
        let query = SemanticQuery {
            query: code_snippet.to_string(),
            file_path: None,
            limit: Some(5),
        };

        self.semantic_search(&query)
    }

    pub fn get_contextual_recommendations(
        &self,
        context: &str,
        file_path: Option<&str>,
    ) -> Result<Vec<SearchResult>, String> {
        let query = SemanticQuery {
            query: context.to_string(),
            file_path: file_path.map(|s| s.to_string()),
            limit: Some(3),
        };

        self.semantic_search(&query)
    }

    pub fn update_file_index(&self, file_path: &str) -> Result<(), String> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let mut chunks = self.chunks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Remove old chunks for this file
        chunks.retain(|_, chunk| chunk.file_path != file_path);

        // Add new chunks
        let new_chunks = self.chunk_file(&content, file_path);
        for chunk in new_chunks {
            chunks.insert(chunk.id.clone(), chunk);
        }

        stats.total_chunks = chunks.len();
        stats.index_size_bytes = Self::estimate_index_size(&chunks);
        stats.last_updated = chrono::Utc::now().timestamp();

        Ok(())
    }

    pub fn get_index_stats(&self) -> Result<IndexStats, String> {
        let stats = self.stats.lock().unwrap();
        Ok(stats.clone())
    }

    pub fn clear_index(&self) -> Result<(), String> {
        let mut chunks = self.chunks.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        chunks.clear();
        stats.total_chunks = 0;
        stats.total_files = 0;
        stats.index_size_bytes = 0;
        stats.last_updated = chrono::Utc::now().timestamp();

        Ok(())
    }

    // Private helper methods

    fn chunk_file(&self, content: &str, file_path: &str) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut current_chunk = String::new();
        let mut chunk_start = 0;
        let mut chunk_count = 0;

        for (i, line) in lines.iter().enumerate() {
            current_chunk.push_str(line);
            current_chunk.push('\n');

            // Create chunk every 50 lines or at function/class boundaries
            if i - chunk_start > 50 || Self::is_chunk_boundary(line) {
                if !current_chunk.trim().is_empty() {
                    let chunk_type = Self::detect_chunk_type(line);
                    let symbol_name = Self::extract_symbol_name(line);
                    let embedding = Self::generate_embedding(&current_chunk);
                    let complexity = Self::calculate_complexity(&current_chunk);
                    let dependencies = Self::extract_dependencies(&current_chunk);

                    chunks.push(CodeChunk {
                        id: format!("{}_{}", file_path, chunk_count),
                        file_path: file_path.to_string(),
                        chunk_type,
                        symbol_name,
                        content: current_chunk.clone(),
                        start_line: chunk_start as u32,
                        end_line: i as u32,
                        embedding,
                        complexity,
                        dependencies,
                    });

                    chunk_count += 1;
                }

                current_chunk.clear();
                chunk_start = i;
            }
        }

        // Add final chunk
        if !current_chunk.trim().is_empty() {
            let embedding = Self::generate_embedding(&current_chunk);
            let complexity = Self::calculate_complexity(&current_chunk);
            let dependencies = Self::extract_dependencies(&current_chunk);

            chunks.push(CodeChunk {
                id: format!("{}_{}", file_path, chunk_count),
                file_path: file_path.to_string(),
                chunk_type: "code".to_string(),
                symbol_name: None,
                content: current_chunk,
                start_line: chunk_start as u32,
                end_line: lines.len() as u32,
                embedding,
                complexity,
                dependencies,
            });
        }

        chunks
    }

    fn is_chunk_boundary(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("def ")
    }

    fn detect_chunk_type(line: &str) -> String {
        let trimmed = line.trim();
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || trimmed.starts_with("async fn ") || trimmed.starts_with("def ") {
            "function".to_string()
        } else if trimmed.starts_with("class ") {
            "class".to_string()
        } else if trimmed.starts_with("interface ") {
            "interface".to_string()
        } else if trimmed.starts_with("impl ") {
            "implementation".to_string()
        } else {
            "code".to_string()
        }
    }

    fn extract_symbol_name(line: &str) -> Option<String> {
        let trimmed = line.trim();
        if let Some(start) = trimmed.find(' ') {
            if let Some(end) = trimmed[start + 1..].find('(') {
                return Some(trimmed[start + 1..start + 1 + end].trim().to_string());
            } else if let Some(end) = trimmed[start + 1..].find('{') {
                return Some(trimmed[start + 1..start + 1 + end].trim().to_string());
            }
        }
        None
    }

    fn calculate_complexity(content: &str) -> u32 {
        let mut complexity = 1u32;
        complexity += content.matches("if ").count() as u32;
        complexity += content.matches("for ").count() as u32;
        complexity += content.matches("while ").count() as u32;
        complexity += content.matches("match ").count() as u32;
        complexity += content.matches("?").count() as u32;
        complexity
    }

    fn extract_dependencies(content: &str) -> Vec<String> {
        let mut deps = Vec::new();
        for line in content.lines() {
            if line.contains("import ") || line.contains("use ") || line.contains("require(") {
                deps.push(line.trim().to_string());
            }
        }
        deps
    }

    fn generate_embedding(text: &str) -> Vec<f32> {
        // Simple embedding: hash-based vector (in production, use actual ML model)
        let mut embedding = vec![0.0; 384]; // 384-dimensional embedding

        for (i, byte) in text.as_bytes().iter().enumerate() {
            let idx = (i % 384) as usize;
            embedding[idx] += (*byte as f32) / 256.0;
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            embedding.iter_mut().for_each(|x| *x /= norm);
        }

        embedding
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..a.len().min(b.len()) {
            dot_product += a[i] * b[i];
            norm_a += a[i] * a[i];
            norm_b += b[i] * b[i];
        }

        let denominator = (norm_a.sqrt() * norm_b.sqrt()).max(1e-10);
        (dot_product / denominator).max(0.0).min(1.0)
    }

    fn calculate_relevance_score(chunk: &CodeChunk, query: &str, similarity: f32) -> f32 {
        let mut score = similarity;

        // Boost score if query matches symbol name
        if let Some(ref symbol) = chunk.symbol_name {
            if symbol.to_lowercase().contains(&query.to_lowercase()) {
                score *= 1.5;
            }
        }

        // Boost score for lower complexity
        score *= 1.0 / (1.0 + (chunk.complexity as f32 / 10.0));

        score.min(1.0)
    }

    fn estimate_index_size(chunks: &HashMap<String, CodeChunk>) -> usize {
        chunks
            .values()
            .map(|chunk| {
                chunk.content.len()
                    + chunk.embedding.len() * 4
                    + chunk.dependencies.iter().map(|d| d.len()).sum::<usize>()
            })
            .sum()
    }
}

// Tauri Commands

#[tauri::command]
pub fn vector_index_workspace(
    workspace_path: String,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<(), String> {
    let system = state.lock().unwrap();
    system.index_workspace(&workspace_path)
}

#[tauri::command]
pub fn vector_semantic_search(
    query: String,
    file_path: Option<String>,
    limit: Option<usize>,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<SearchResult>, String> {
    let system = state.lock().unwrap();
    let search_query = SemanticQuery {
        query,
        file_path,
        limit,
    };
    system.semantic_search(&search_query)
}

#[tauri::command]
pub fn vector_find_similar(
    code_snippet: String,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<SearchResult>, String> {
    let system = state.lock().unwrap();
    system.find_similar_code(&code_snippet)
}

#[tauri::command]
pub fn vector_get_recommendations(
    context: String,
    file_path: Option<String>,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<SearchResult>, String> {
    let system = state.lock().unwrap();
    system.get_contextual_recommendations(&context, file_path.as_deref())
}

#[tauri::command]
pub fn vector_update_file(
    file_path: String,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<(), String> {
    let system = state.lock().unwrap();
    system.update_file_index(&file_path)
}

#[tauri::command]
pub fn vector_get_stats(
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<IndexStats, String> {
    let system = state.lock().unwrap();
    system.get_index_stats()
}

#[tauri::command]
pub fn vector_clear_index(
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<(), String> {
    let system = state.lock().unwrap();
    system.clear_index()
}
