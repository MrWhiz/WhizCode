use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: String,
    pub file_path: String,
    pub chunk_type: String,
    pub symbol_name: Option<String>,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: CodeChunk,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticQuery {
    pub query: String,
    pub file_path: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexStats {
    pub total_chunks: usize,
    pub total_files: usize,
    pub total_symbols: usize,
    pub last_index_time: Option<u64>,
    pub is_indexing: bool,
}

pub struct VectorSearchSystem {
    workspace_root: PathBuf,
    stats: std::sync::Arc<std::sync::Mutex<IndexStats>>,
    chunks_by_file: std::sync::Arc<std::sync::Mutex<HashMap<String, Vec<CodeChunk>>>>,
    file_signatures: std::sync::Arc<std::sync::Mutex<HashMap<String, u64>>>,
}

impl VectorSearchSystem {
    pub fn new(workspace_path: &str) -> Result<Self> {
        Ok(Self {
            workspace_root: PathBuf::from(workspace_path),
            stats: std::sync::Arc::new(std::sync::Mutex::new(IndexStats::default())),
            chunks_by_file: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            file_signatures: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        })
    }

    fn current_file_signature(path: &Path) -> u64 {
        let size = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
        let modified = std::fs::metadata(path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        size ^ modified
    }

    pub fn build_file_tree(workspace_path: &str, max_files: usize) -> String {
        let root = Path::new(workspace_path);
        if !root.exists() {
            return String::new();
        }

        WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !is_ignored_path(entry.path()))
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .take(max_files)
            .filter_map(|entry| {
                entry
                    .path()
                    .strip_prefix(root)
                    .ok()
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn index_workspace(&mut self, workspace_path: &str) -> Result<()> {
        self.workspace_root = PathBuf::from(workspace_path);
        {
            let mut stats = self.stats.lock().unwrap();
            stats.is_indexing = true;
        }

        let mut chunks_by_file = HashMap::new();
        let mut file_signatures = HashMap::new();
        let mut total_chunks = 0usize;
        let mut total_symbols = HashSet::new();

        for entry in WalkDir::new(&self.workspace_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !is_ignored_path(entry.path()))
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file() && is_text_candidate(entry.path()))
        {
            let path = entry.path().to_path_buf();
            let relative_path = path
                .strip_prefix(&self.workspace_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let chunks = collect_chunks(&self.workspace_root, Some(path.clone()))?;
            for chunk in &chunks {
                if let Some(symbol_name) = &chunk.symbol_name {
                    total_symbols.insert(symbol_name.clone());
                }
            }
            total_chunks += chunks.len();
            file_signatures.insert(relative_path.clone(), Self::current_file_signature(&path));
            chunks_by_file.insert(relative_path, chunks);
        }

        {
            let mut cache = self.chunks_by_file.lock().unwrap();
            *cache = chunks_by_file;
        }
        {
            let mut signatures = self.file_signatures.lock().unwrap();
            *signatures = file_signatures;
        }
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_chunks = total_chunks;
            stats.total_files = self.chunks_by_file.lock().unwrap().len();
            stats.total_symbols = total_symbols.len();
            stats.last_index_time = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .map(|duration| duration.as_secs())
                    .unwrap_or_default(),
            );
            stats.is_indexing = false;
        }

        Ok(())
    }

    pub fn semantic_search(&self, query: &SemanticQuery) -> Result<Vec<SearchResult>> {
        let limit = query.limit.unwrap_or(5);
        let chunks = {
            let cache = self.chunks_by_file.lock().unwrap();
            if cache.is_empty() {
                drop(cache);
                collect_chunks(
                    &self.workspace_root,
                    query.file_path.as_deref().map(PathBuf::from),
                )?
            } else if let Some(file_path) = query.file_path.as_deref() {
                let normalized = if Path::new(file_path).is_absolute() {
                    Path::new(file_path)
                        .strip_prefix(&self.workspace_root)
                        .unwrap_or(Path::new(file_path))
                        .to_string_lossy()
                        .replace('\\', "/")
                } else {
                    file_path.replace('\\', "/")
                };
                cache.get(&normalized).cloned().unwrap_or_default()
            } else {
                cache.values().flat_map(|items| items.clone()).collect::<Vec<_>>()
            }
        };
        let query_terms = tokenize(&query.query);

        let mut results = chunks
            .into_iter()
            .filter_map(|chunk| {
                let score = lexical_score(&query_terms, &chunk.content, &chunk.file_path);
                if score > 0.0 {
                    Some(SearchResult {
                        chunk,
                        relevance_score: score,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        Ok(results)
    }

    pub fn update_file(&mut self, path: &str) -> Result<()> {
        let resolved = {
            let candidate = PathBuf::from(path);
            if candidate.is_absolute() {
                candidate
            } else {
                self.workspace_root.join(candidate)
            }
        };

        let relative_path = resolved
            .strip_prefix(&self.workspace_root)
            .unwrap_or(&resolved)
            .to_string_lossy()
            .replace('\\', "/");

        if !resolved.exists() {
            self.remove_file(path)?;
            return Ok(());
        }

        if !resolved.is_file() || !is_text_candidate(&resolved) {
            return Ok(());
        }

        let chunks = collect_chunks(&self.workspace_root, Some(resolved.clone()))?;
        let signature = Self::current_file_signature(&resolved);
        {
            let mut cache = self.chunks_by_file.lock().unwrap();
            cache.insert(relative_path.clone(), chunks);
        }
        {
            let mut signatures = self.file_signatures.lock().unwrap();
            signatures.insert(relative_path, signature);
        }

        self.recalculate_stats();

        Ok(())
    }

    pub fn clear_index(&mut self) -> Result<()> {
        {
            let mut stats = self.stats.lock().unwrap();
            *stats = IndexStats::default();
        }
        self.chunks_by_file.lock().unwrap().clear();
        self.file_signatures.lock().unwrap().clear();
        Ok(())
    }

    pub fn get_index_stats(&self) -> Result<IndexStats> {
        Ok(self.stats.lock().unwrap().clone())
    }

    pub fn remove_file(&mut self, path: &str) -> Result<()> {
        let relative_path = {
            let candidate = PathBuf::from(path);
            if candidate.is_absolute() {
                candidate
                    .strip_prefix(&self.workspace_root)
                    .unwrap_or(&candidate)
                    .to_string_lossy()
                    .replace('\\', "/")
            } else {
                candidate.to_string_lossy().replace('\\', "/")
            }
        };

        self.chunks_by_file.lock().unwrap().remove(&relative_path);
        self.file_signatures.lock().unwrap().remove(&relative_path);
        self.recalculate_stats();
        Ok(())
    }

    fn recalculate_stats(&self) {
        let cache = self.chunks_by_file.lock().unwrap();
        let mut symbol_names = HashSet::new();
        let mut total_chunks = 0usize;
        for chunks in cache.values() {
            total_chunks += chunks.len();
            for chunk in chunks {
                if let Some(symbol_name) = &chunk.symbol_name {
                    symbol_names.insert(symbol_name.clone());
                }
            }
        }

        let mut stats = self.stats.lock().unwrap();
        stats.total_chunks = total_chunks;
        stats.total_files = cache.len();
        stats.total_symbols = symbol_names.len();
        stats.last_index_time = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_secs())
                .unwrap_or_default(),
        );
        stats.is_indexing = false;
    }
}

fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| {
        let part = component.as_os_str().to_string_lossy().to_lowercase();
        matches!(
            part.as_str(),
            ".git" | "node_modules" | ".whizcode" | "target" | "dist" | "build" | ".next" | "coverage"
        )
    })
}

fn is_text_candidate(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    matches!(
        extension.as_deref(),
        Some(
            "rs"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "json"
                | "md"
                | "py"
                | "go"
                | "java"
                | "kt"
                | "swift"
                | "css"
                | "scss"
                | "html"
                | "yml"
                | "yaml"
                | "toml"
                | "sh"
                | "ps1"
        )
    )
}

fn collect_chunks(root: &Path, file_filter: Option<PathBuf>) -> Result<Vec<CodeChunk>> {
    let target = file_filter.and_then(|path| {
        if path.is_absolute() {
            Some(path)
        } else {
            Some(root.join(path))
        }
    });

    let files = if let Some(target_path) = target {
        if target_path.is_file() {
            vec![target_path]
        } else {
            Vec::new()
        }
    } else {
        WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !is_ignored_path(entry.path()))
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file() && is_text_candidate(entry.path()))
            .map(|entry| entry.path().to_path_buf())
            .collect::<Vec<_>>()
    };

    let mut chunks = Vec::new();

    for file_path in files {
        let content = match std::fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let relative_path = file_path
            .strip_prefix(root)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .replace('\\', "/");

        let lines = content.lines().collect::<Vec<_>>();
        if lines.is_empty() {
            continue;
        }

        let mut start = 0usize;
        while start < lines.len() {
            let end = (start + 120).min(lines.len());
            let chunk_lines = &lines[start..end];
            let chunk_content = chunk_lines.join("\n");

            chunks.push(CodeChunk {
                id: format!("{}:{}-{}", relative_path, start + 1, end),
                file_path: relative_path.clone(),
                chunk_type: "file_chunk".to_string(),
                symbol_name: detect_symbol(chunk_lines),
                content: chunk_content,
                start_line: start + 1,
                end_line: end,
            });

            start = end;
        }
    }

    Ok(chunks)
}

fn detect_symbol(lines: &[&str]) -> Option<String> {
    for line in lines {
        let trimmed = line.trim();
        let candidates = [
            "fn ", "function ", "class ", "interface ", "type ", "struct ", "enum ", "const ",
        ];
        for prefix in candidates {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let name = rest
                    .split(|character: char| !character.is_alphanumeric() && character != '_' && character != '$')
                    .find(|segment| !segment.is_empty())?;
                return Some(name.to_string());
            }
        }
    }

    None
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .split(|character: char| !character.is_alphanumeric() && character != '_' && character != '$')
        .filter(|token| token.len() >= 2)
        .map(|token| token.to_lowercase())
        .collect()
}

fn lexical_score(query_terms: &[String], content: &str, file_path: &str) -> f32 {
    if query_terms.is_empty() {
        return 0.0;
    }

    let haystack = format!("{}\n{}", file_path.to_lowercase(), content.to_lowercase());
    let mut matched = 0f32;
    let mut density = 0f32;

    for term in query_terms {
        if haystack.contains(term) {
            matched += 1.0;
            density += haystack.matches(term).count() as f32;
        }
    }

    if matched == 0.0 {
        0.0
    } else {
        (matched / query_terms.len() as f32) + (density.min(6.0) * 0.05)
    }
}

#[tauri::command]
pub async fn vector_index_workspace(
    workspace_path: String,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<()> {
    let mut system = state.lock().unwrap();
    system.index_workspace(&workspace_path)
}

#[tauri::command]
pub async fn vector_index_workspace_full(
    workspace_path: String,
    _mode: Option<String>,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<()> {
    let mut system = state.lock().unwrap();
    system.index_workspace(&workspace_path)
}

#[tauri::command]
pub async fn vector_get_file_tree(
    workspace_path: String,
    max_files: Option<usize>,
) -> Result<String> {
    Ok(VectorSearchSystem::build_file_tree(
        &workspace_path,
        max_files.unwrap_or(300),
    ))
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
    system.get_index_stats()
}

#[tauri::command]
pub async fn vector_find_similar(
    content: String,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<SearchResult>> {
    let system = state.lock().unwrap();
    system.semantic_search(&SemanticQuery {
        query: content,
        file_path: None,
        limit: Some(5),
    })
}

#[tauri::command]
pub async fn vector_get_recommendations(
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<Vec<String>> {
    let system = state.lock().unwrap();
    let stats = system.get_index_stats()?;
    let mut recommendations = Vec::new();

    if stats.total_files == 0 {
        recommendations.push("Workspace search has not been scanned yet.".to_string());
    } else {
        recommendations.push("Workspace lexical search is available for file and symbol narrowing.".to_string());
    }

    if stats.total_chunks < 100 {
        recommendations.push("Run a workspace scan after large repo changes to refresh search statistics.".to_string());
    }

    Ok(recommendations)
}

#[tauri::command]
pub async fn vector_update_file(
    path: String,
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<()> {
    let mut system = state.lock().unwrap();
    system.update_file(&path)
}

#[tauri::command]
pub async fn vector_get_stats(
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<IndexStats> {
    let system = state.lock().unwrap();
    system.get_index_stats()
}

#[tauri::command]
pub async fn vector_clear_index(
    state: State<'_, Arc<Mutex<VectorSearchSystem>>>,
) -> Result<()> {
    let mut system = state.lock().unwrap();
    system.clear_index()
}
