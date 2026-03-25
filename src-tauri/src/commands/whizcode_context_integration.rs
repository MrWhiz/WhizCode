/**
 * WhizCode Context Integration
 * Integrates context optimization into the agent streaming pipeline
 */

use crate::commands::context_optimizer::ContextOptimizer;
use std::path::Path;

/// Load workspace files for context optimization
#[allow(dead_code)]
pub fn load_workspace_files(workspace_path: &str) -> Result<Vec<(String, String)>, String> {
    let files = Vec::new();
    let _path = Path::new(workspace_path);

    // Load key files from workspace
    let _key_patterns = vec![
        "package.json",
        "tsconfig.json",
        "README.md",
        "src/**/*.ts",
        "src/**/*.tsx",
        "src/**/*.js",
        "src/**/*.jsx",
    ];

    // For now, return empty list - full implementation in Phase 2
    // This will be enhanced to actually load files from the workspace
    Ok(files)
}

/// Optimize context for local LLM
#[allow(dead_code)]
pub fn optimize_context_for_llm(
    files: Vec<(String, String)>,
    query: &str,
    workspace_path: &str,
    max_tokens: u32,
) -> Result<String, String> {
    let mut optimizer = ContextOptimizer::new(Some(max_tokens));
    let pruned = optimizer.prune_context(files, query, workspace_path);

    // Build optimized context string
    let mut context = String::new();
    context.push_str("## Optimized Context\n\n");
    context.push_str(&format!("Total files: {}\n", pruned.files.len()));
    context.push_str(&format!("Total tokens: {}\n", pruned.estimated_tokens));
    context.push_str(&format!("Token reduction: {:.1}%\n\n", 
        (1.0 - (pruned.estimated_tokens as f32 / max_tokens as f32)) * 100.0));

    context.push_str("## Files Included\n\n");
    for file in &pruned.files {
        context.push_str(&format!("- {} ({} tokens, {})\n", 
            file.path, file.estimated_tokens, file.file_type));
    }

    context.push_str("\n## Context Summary\n\n");
    context.push_str(&pruned.summary);

    Ok(context)
}

/// Calculate context reduction percentage
#[allow(dead_code)]
pub fn calculate_context_reduction(original_tokens: u32, optimized_tokens: u32) -> f32 {
    if original_tokens == 0 {
        return 0.0;
    }
    ((original_tokens - optimized_tokens) as f32 / original_tokens as f32) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_reduction_calculation() {
        let reduction = calculate_context_reduction(1000, 700);
        assert_eq!(reduction, 30.0);
    }

    #[test]
    fn test_context_reduction_zero_original() {
        let reduction = calculate_context_reduction(0, 0);
        assert_eq!(reduction, 0.0);
    }
}
