/**
 * WhizCode Tauri Commands
 * Exposes WhizCode integration layer functions as Tauri commands
 */

use crate::error::Result;
use crate::commands::whizcode_integration::{
    WhizCodeIntegrationLayer, QueryAnalysis, OptimizedPrompt, PrunedContext, WorkflowRoute,
};
// Tauri command wrappers for WhizCode integration

/// Analyze a user query and classify it
#[tauri::command]
pub fn analyze_query(query: String) -> Result<QueryAnalysis> {
    let analysis = WhizCodeIntegrationLayer::analyze_query(&query);
    Ok(analysis)
}

/// Generate an optimized prompt for local LLM
#[tauri::command]
pub fn generate_optimized_prompt(
    query: String,
    query_type: String,
    context_size: usize,
) -> Result<OptimizedPrompt> {
    let prompt = WhizCodeIntegrationLayer::generate_optimized_prompt(&query, &query_type, context_size);
    Ok(prompt)
}

/// Prune context to fit token limit
#[tauri::command]
pub fn optimize_context(
    files: Vec<(String, String)>,
    query: String,
    max_tokens: u32,
) -> Result<PrunedContext> {
    let pruned = WhizCodeIntegrationLayer::prune_context(files, &query, max_tokens);
    Ok(pruned)
}

/// Route query to appropriate workflow
#[tauri::command]
pub fn route_query(query: String, query_type: String) -> Result<WorkflowRoute> {
    let route = WhizCodeIntegrationLayer::route_query(&query, &query_type);
    Ok(route)
}

/// Get streaming metrics
#[tauri::command]
pub fn get_streaming_metrics() -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "status": "ready",
        "message": "Streaming metrics endpoint ready"
    }))
}
