use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunedFile {
    pub path: String,
    pub estimated_tokens: u32,
    pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunedContext {
    pub files: Vec<PrunedFile>,
    pub estimated_tokens: u32,
    pub summary: String,
}

pub struct ContextOptimizer {
    max_context_size: u32,
}

impl ContextOptimizer {
    pub fn new(max_context_size: Option<u32>) -> Self {
        Self {
            max_context_size: max_context_size.unwrap_or(8000),
        }
    }

    pub fn prune_context(
        &mut self,
        files: Vec<(String, String)>,
        query: &str,
        _workspace_path: &str,
    ) -> PrunedContext {
        let mut pruned_files = Vec::new();
        let mut total_tokens = 0u32;
        let mut file_relevance: HashMap<String, f32> = HashMap::new();

        // Calculate relevance scores based on query keywords
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        for (path, content) in &files {
            let path_lower = path.to_lowercase();
            let content_lower = content.to_lowercase();
            
            // Calculate relevance score
            let mut relevance = 0.0f32;
            
            // Path matches are highly relevant
            for word in &query_words {
                if path_lower.contains(word) {
                    relevance += 2.0;
                }
            }
            
            // Content matches
            for word in &query_words {
                let count = content_lower.matches(word).count() as f32;
                relevance += count * 0.1;
            }
            
            file_relevance.insert(path.clone(), relevance);
        }

        // Sort files by relevance
        let mut sorted_files: Vec<_> = files.into_iter().collect();
        sorted_files.sort_by(|a, b| {
            let relevance_a = file_relevance.get(&a.0).copied().unwrap_or(0.0);
            let relevance_b = file_relevance.get(&b.0).copied().unwrap_or(0.0);
            relevance_b.partial_cmp(&relevance_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Prune files to fit within context size
        for (path, content) in sorted_files {
            let estimated_tokens = (content.len() / 4) as u32; // Rough estimate: 4 chars per token
            
            if total_tokens + estimated_tokens <= self.max_context_size {
                let file_type = path.split('.').last().unwrap_or("unknown").to_string();
                pruned_files.push(PrunedFile {
                    path: path.clone(),
                    estimated_tokens,
                    file_type,
                });
                total_tokens += estimated_tokens;
            }
        }

        let summary = format!(
            "Pruned context to {} files with ~{} tokens (max: {})",
            pruned_files.len(),
            total_tokens,
            self.max_context_size
        );

        PrunedContext {
            files: pruned_files,
            estimated_tokens: total_tokens,
            summary,
        }
    }
}
