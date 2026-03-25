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
        .manage(Arc::new(std::sync::Mutex::new(commands::terminal::TerminalManager::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::process::ProcessManager::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::tool_result_cache::ToolResultCache::new(None))))
        .manage(Arc::new(std::sync::Mutex::new(
            commands::vector_search::VectorSearchSystem::new(".")
                .unwrap_or_else(|_| commands::vector_search::VectorSearchSystem::new(".").unwrap())
        )))
        .manage(Arc::new(std::sync::Mutex::new(commands::error_recovery::ErrorRecoverySystem::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::mcp_service::MCPService::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::learning::LearningSystem::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::context_memory::ContextMemory::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::graph::GraphService::new())))
        .manage(Arc::new(RwLock::new(commands::steering::SteeringSystem::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::index::IndexService::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::diagnostics_service::DiagnosticsService::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::diff::DiffService::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::memory::MemoryService::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::hooks::HooksManager::new())))
        .manage(Arc::new(std::sync::Mutex::new(commands::code_intelligence::CodeIntelligence::new())))
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
            commands::fs::watch_directory,
            
            // Terminal operations
            commands::terminal::terminal_create,
            commands::terminal::terminal_list,
            commands::terminal::terminal_get,
            commands::terminal::terminal_close,
            commands::terminal::terminal_get_available_shells,
            commands::terminal::terminal_get_default_shell,
            commands::terminal::terminal_write,
            commands::terminal::terminal_resize,
            
            // Process operations
            commands::process::process_check,
            commands::process::process_stop,
            commands::process::process_list,
            commands::process::process_summary,
            commands::process::process_clear,
            
            // History operations
            commands::history::history_save,
            commands::history::history_list,
            commands::history::history_get,
            commands::history::history_delete,
            commands::history::history_search,
            commands::history::history_update,
            commands::history::history_clear_cache,
            
            // Tool result cache operations
            commands::tool_result_cache::cache_get,
            commands::tool_result_cache::cache_set,
            commands::tool_result_cache::cache_invalidate,
            commands::tool_result_cache::cache_cleanup,
            commands::tool_result_cache::cache_clear,
            commands::tool_result_cache::cache_get_stats,
            
            // Vector search operations
            commands::vector_search::vector_index_workspace,
            commands::vector_search::vector_index_workspace_full,
            commands::vector_search::vector_get_file_tree,
            commands::vector_search::vector_semantic_search,
            commands::vector_search::vector_find_similar,
            commands::vector_search::vector_get_recommendations,
            commands::vector_search::vector_update_file,
            commands::vector_search::vector_get_stats,
            commands::vector_search::vector_clear_index,
            
            // Error recovery operations
            commands::error_recovery::error_recovery_handle,
            commands::error_recovery::error_recovery_history,
            commands::error_recovery::error_recovery_strategies,
            commands::error_recovery::error_recovery_statistics,
            commands::error_recovery::error_recovery_clear_history,
            commands::error_recovery::error_recovery_add_strategy,
            commands::error_recovery::error_recovery_remove_strategy,
            commands::error_recovery::error_recovery_auto_recover,
            commands::error_recovery::error_recovery_execute_strategy,
            commands::error_recovery::error_recovery_get_log,
            commands::error_recovery::error_recovery_strategy_effectiveness,
            commands::error_recovery::error_recovery_update_strategy_rates,
            commands::error_recovery::error_recovery_best_strategy,
            
            // MCP service operations
            commands::mcp_service::mcp_initialize,
            commands::mcp_service::mcp_add_server,
            commands::mcp_service::mcp_remove_server,
            commands::mcp_service::mcp_enable_server,
            commands::mcp_service::mcp_disable_server,
            commands::mcp_service::mcp_get_servers,
            commands::mcp_service::mcp_get_server_status,
            commands::mcp_service::mcp_get_all_server_status,
            commands::mcp_service::mcp_register_tool,
            commands::mcp_service::mcp_get_tools,
            commands::mcp_service::mcp_get_tools_by_server,
            commands::mcp_service::mcp_call_tool,
            commands::mcp_service::mcp_get_marketplace,
            commands::mcp_service::mcp_install_power,
            commands::mcp_service::mcp_uninstall_power,
            commands::mcp_service::mcp_add_configuration,
            commands::mcp_service::mcp_get_configurations,
            commands::mcp_service::mcp_validate_configuration,
            commands::mcp_service::mcp_get_metrics,
            commands::mcp_service::mcp_clear_tools,
            commands::mcp_service::mcp_build_tool_prompt,
            
            // Search operations
            commands::search::search_files,
            commands::search::fuzzy_find_file,
            
            // System operations
            commands::system::get_system_info,
            commands::system::open_external,
            commands::system::reveal_in_folder,
            commands::system::open_terminal_at,
            
            // Workspace operations
            commands::workspace::set_workspace,
            commands::workspace::get_workspace,
            
            // Ollama operations
            commands::ollama::ollama_health_check,
            commands::ollama::ollama_get_models,
            commands::ollama::ollama_pull_model,
            
            // Dialog operations
            commands::dialog::dialog_open_folder,
            
            // Agent operations
            commands::agent::execute_agent_task,
            commands::agent::agent_stop,
            commands::agent::agent_reset,
            commands::agent::agent_permission_response,
            
            // Git operations
            commands::git::git_status,
            commands::git::git_stage,
            commands::git::git_commit,
            commands::git::git_review,
            
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
            commands::distillation::distill_session,
            commands::assets::generate_image,
            commands::workflows::list_workflows,
            commands::workflows::list_skills,
            
            // Web Search operations
            commands::web_search::search_web,
            commands::web_search::read_url_content,
            
            // Vector operations
            commands::vector_search::vector_get_index_stats,
            commands::vector_search::vector_index_workspace_full,
            commands::vector_search::vector_semantic_search,
            commands::vector_search::vector_get_file_tree,
            commands::vector_search::vector_update_file,
            commands::vector_search::vector_clear_index,
            commands::vector_search::vector_get_stats,
            
            // Specs operations
            commands::specs::specs_list,
            commands::specs::specs_get,
            
            // Planning operations
            commands::planning::create_execution_plan,
            
            // Sub-agents operations
            commands::sub_agents::list_sub_agents,
            commands::sub_agents::get_sub_agent_config,
            commands::sub_agents::invoke_sub_agent,
            commands::sub_agents::orchestrate_sub_agents,
            
            // Learning operations
            commands::learning::learning_analyze_patterns,
            commands::learning::learning_get_recommendations,
            commands::learning::learning_get_metrics,
            commands::learning::learning_record_interaction,
            commands::learning::learning_get_insights,
            commands::learning::learning_clear_history,
            
            // Context memory operations
            commands::context_memory::context_memory_record_pattern,
            commands::context_memory::context_memory_get_patterns,
            commands::context_memory::context_memory_record_preference,
            commands::context_memory::context_memory_get_preference,
            commands::context_memory::context_memory_get_all_preferences,
            commands::context_memory::context_memory_record_error,
            commands::context_memory::context_memory_get_similar_errors,
            commands::context_memory::context_memory_get_all_errors,
            commands::context_memory::context_memory_record_strategy,
            commands::context_memory::context_memory_get_best_strategies,
            commands::context_memory::context_memory_get_all_strategies,
            commands::context_memory::context_memory_record_project,
            commands::context_memory::context_memory_get_project,
            commands::context_memory::context_memory_get_all_projects,
            commands::context_memory::context_memory_get_statistics,
            commands::context_memory::context_memory_clear_old_data,
            commands::context_memory::context_memory_get_snapshot,
            commands::context_memory::context_memory_delete_preference,
            commands::context_memory::context_memory_delete_project,
            
            // Hooks operations
            commands::hooks::hooks_list_all,
            commands::hooks::hooks_get_enabled,
            commands::hooks::hooks_add,
            commands::hooks::hooks_remove,
            commands::hooks::hooks_update,
            commands::hooks::hooks_get_for_event,
            commands::hooks::hooks_trigger_file_event,
            commands::hooks::hooks_trigger_tool_event,
            commands::hooks::hooks_enable,
            commands::hooks::hooks_disable,
            commands::hooks::hooks_get_execution_history,
            commands::hooks::hooks_get_metrics,
            commands::hooks::hooks_clear_execution_history,
            
            // Steering operations
            commands::steering::steering_add_file,
            commands::steering::steering_remove_file,
            commands::steering::steering_get_file,
            commands::steering::steering_list_all,
            commands::steering::steering_get_enabled,
            commands::steering::steering_enable_file,
            commands::steering::steering_disable_file,
            commands::steering::steering_update_file,
            commands::steering::steering_load_context,
            commands::steering::steering_get_injected_context,
            commands::steering::steering_get_metrics,
            commands::steering::steering_clear_context,
            
            // Index operations
            commands::index::index_build_index,
            commands::index::index_search_files,
            commands::index::index_search_symbols,
            commands::index::index_get_file_symbols,
            commands::index::index_update_file,
            commands::index::index_remove_file,
            commands::index::index_get_stats,
            commands::index::index_clear,
            
            // Diagnostics operations
            commands::diagnostics_service::diagnostics_check_file,
            commands::diagnostics_service::diagnostics_get_report,
            commands::diagnostics_service::diagnostics_get_all_reports,
            commands::diagnostics_service::diagnostics_get_stats,
            commands::diagnostics_service::diagnostics_clear_reports,
            commands::diagnostics_service::diagnostics_get_history,
            
            // Diff operations
            commands::diff::diff_generate,
            commands::diff::diff_record_change,
            commands::diff::diff_get_file_history,
            commands::diff::diff_get_all_changes,
            commands::diff::diff_rollback_change,
            commands::diff::diff_get_stats,
            commands::diff::diff_clear_history,
            
            // Memory operations
            commands::memory::memory_get_stats,
            commands::memory::memory_detect_leaks,
            commands::memory::memory_run_gc,
            commands::memory::memory_cleanup_old,
            commands::memory::memory_get_allocations,
            commands::memory::memory_clear,
            commands::code_intelligence::code_intelligence_analyze_workspace,
            commands::code_intelligence::code_intelligence_get_symbol_info,
            commands::code_intelligence::code_intelligence_find_related_symbols,
            commands::code_intelligence::code_intelligence_suggest_refactoring,
            commands::code_intelligence::code_intelligence_get_metrics,
            commands::code_intelligence::code_intelligence_get_all_symbols,
            commands::code_intelligence::code_intelligence_get_all_relationships,
            commands::code_intelligence::code_intelligence_get_all_patterns,
            commands::code_intelligence::code_intelligence_find_circular_dependencies,
            commands::code_intelligence::code_intelligence_impact_analysis,
            
            // Graph operations
            commands::graph::graph_build_dependency_graph,
            commands::graph::graph_find_circular_dependencies,
            commands::graph::graph_analyze_impact,
            commands::graph::graph_analyze_reachability,
            commands::graph::graph_get_graph,
            commands::graph::graph_clear_graph,
            
            // Agent orchestrator operations
            commands::agent_orchestrator::execute_agent_loop,
            commands::agent_orchestrator::agent_reasoning_with_cot,
            commands::agent_orchestrator::agent_validate_cot_response,
            commands::agent_orchestrator::agent_get_cot_metrics,
            commands::agent_orchestrator::agent_evaluate_confidence,
            commands::agent_orchestrator::agent_calculate_tool_confidence,
            commands::agent_orchestrator::agent_assess_decision_risk,
            commands::agent_orchestrator::agent_get_confidence_thresholds,
            
            // Tool metrics operations
            commands::tool_metrics::tool_metrics_record_execution,
            commands::tool_metrics::tool_metrics_get_metrics,
            commands::tool_metrics::tool_metrics_get_all,
            commands::tool_metrics::tool_metrics_rank_tools,
            commands::tool_metrics::tool_metrics_get_recommendations,
            commands::tool_metrics::tool_metrics_get_history,
            commands::tool_metrics::tool_metrics_get_failure_analysis,
            commands::tool_metrics::tool_metrics_get_statistics,
            commands::tool_metrics::tool_metrics_clear,
            
            // Agent streaming operations
            commands::agent_streaming::execute_agent_loop_streaming,
            commands::agent_streaming::agent_send_terminal_input,
            commands::agent_streaming::agent_stop_terminal_command,
            
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
            
            // WhizCode operations
            commands::whizcode_commands::analyze_query,
            commands::whizcode_commands::generate_optimized_prompt,
            commands::whizcode_commands::optimize_context,
            commands::whizcode_commands::route_query,
            commands::whizcode_commands::get_streaming_metrics,
            
            // Task management operations
            commands::task_commands::get_task_progress,
            commands::task_commands::get_tasks_by_status,
            commands::task_commands::update_task_status,
            commands::task_commands::load_tasks_markdown,
            commands::task_commands::tasks_exist,
            commands::task_commands::get_pending_tasks_count,
            commands::task_commands::get_completed_tasks_count,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Initialize app state
            let state = app.state::<Arc<RwLock<AppState>>>();
            let mut app_state = state.write();
            app_state.app_handle = Some(app_handle.clone());
            
            // Initialize history service with default workspace
            let workspace_path = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());
            
            if let Ok(history_service) = commands::history::HistoryService::new(&workspace_path) {
                app.manage(Arc::new(std::sync::Mutex::new(history_service)));
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
