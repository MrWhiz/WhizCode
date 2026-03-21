#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod error;
mod utils;

use tauri::Manager;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(AppState::new())))
        .invoke_handler(tauri::generate_handler![
            // File operations
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::read_directory,
            commands::fs::read_directory_recursive,
            commands::fs::create_file,
            commands::fs::create_directory,
            commands::fs::delete_file,
            commands::fs::delete_directory,
            commands::fs::rename_file,
            commands::fs::check_file_exists,
            
            // Terminal operations
            commands::terminal::create_terminal,
            commands::terminal::write_to_terminal,
            commands::terminal::resize_terminal,
            commands::terminal::close_terminal,
            commands::terminal::get_available_shells,
            commands::terminal::get_default_shell,
            
            // Search operations
            commands::search::search_files,
            commands::search::fuzzy_find_file,
            
            // System operations
            commands::system::get_system_info,
            commands::system::open_external,
            
            // Workspace operations
            commands::workspace::set_workspace,
            commands::workspace::get_workspace,
            
            // Ollama operations
            commands::ollama::ollama_health_check,
            commands::ollama::ollama_get_models,
            
            // Dialog operations
            commands::dialog::dialog_open_folder,
            
            // Agent operations
            commands::agent::execute_agent_task,
            commands::agent::agent_stop,
            commands::agent::agent_reset,
            commands::agent::agent_permission_response,
            
            // Git operations
            commands::git::git_status,
            
            // Diagnostics operations
            commands::diagnostics::diagnostics_check,
            
            // Azure operations
            commands::azure::azure_get_token_status,
            commands::azure::azure_generate_token,
            
            // AI operations
            commands::ai::ai_get_learning_insights,
            commands::ai::ai_get_learning_metrics,
            commands::ai::ai_get_code_metrics,
            commands::ai::ai_get_context_memory_stats,
            
            // Vector operations
            commands::vector::vector_get_index_stats,
            
            // Cache operations
            commands::cache::cache_get_stats,
            
            // Error recovery operations
            commands::error_recovery::error_recovery_get_statistics,
            
            // MCP operations
            commands::mcp::mcp_get_marketplace,
            
            // Specs operations
            commands::specs::specs_list,
            commands::specs::specs_get,
            
            // Planner operations
            commands::planner::create_plan,
            
            // Sub-agents operations
            commands::sub_agents::list_sub_agents,
            commands::sub_agents::get_sub_agent_config,
            commands::sub_agents::invoke_sub_agent,
            
            // Learning operations
            commands::learning::learning_analyze_patterns,
            commands::learning::learning_get_recommendations,
            commands::learning::learning_get_metrics,
            commands::learning::learning_record_interaction,
            
            // Context memory operations
            commands::context_memory::context_memory_record_pattern,
            commands::context_memory::context_memory_get_patterns,
            commands::context_memory::context_memory_record_preference,
            commands::context_memory::context_memory_get_preference,
            commands::context_memory::context_memory_record_error,
            commands::context_memory::context_memory_get_similar_errors,
            commands::context_memory::context_memory_record_strategy,
            commands::context_memory::context_memory_get_best_strategies,
            
            // Hooks operations
            commands::hooks::hooks_list_all,
            commands::hooks::hooks_get_enabled,
            commands::hooks::hooks_add,
            commands::hooks::hooks_remove,
            commands::hooks::hooks_update,
            commands::hooks::hooks_get_for_event,
            commands::hooks::hooks_trigger_file_event,
            commands::hooks::hooks_trigger_tool_event,
            
            // Code intelligence operations
            commands::code_intelligence::code_intelligence_analyze_workspace,
            commands::code_intelligence::code_intelligence_get_symbol_info,
            commands::code_intelligence::code_intelligence_find_related_symbols,
            commands::code_intelligence::code_intelligence_suggest_refactoring,
            commands::code_intelligence::code_intelligence_get_metrics,
            
            // Agent orchestrator operations
            commands::agent_orchestrator::execute_agent_loop,
            
            // Agent streaming operations
            commands::agent_streaming::execute_agent_loop_streaming,
            
            // Advanced tools operations
            commands::advanced_tools::execute_edit_file,
            commands::advanced_tools::execute_git_operation,
            commands::advanced_tools::execute_npm_operation,
            commands::advanced_tools::execute_docker_operation,
            
            // Tool cache operations
            commands::tool_cache::tool_cache_get,
            commands::tool_cache::tool_cache_clear,
            commands::tool_cache::tool_cache_get_stats,
            
            // Custom tools operations
            commands::custom_tools::register_custom_tool,
            commands::custom_tools::unregister_custom_tool,
            commands::custom_tools::list_custom_tools,
            commands::custom_tools::execute_custom_tool,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Initialize app state
            let state = app.state::<Arc<RwLock<AppState>>>();
            let mut app_state = state.write();
            app_state.app_handle = Some(app_handle);
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
