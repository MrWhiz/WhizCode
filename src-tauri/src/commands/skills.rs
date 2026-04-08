//! Skills Management System
//!
//! Integrates Claude Code's skills architecture into WhizCode, enabling:
//! - Automatic skill discovery from repositories
//! - Intelligent skill selection based on queries
//! - Efficient caching and performance optimization
//! - Multi-agent orchestration with conflict resolution
//! - Frontend integration via Tauri IPC commands

pub mod models;
pub mod discovery;
pub mod selector;
pub mod cache;
pub mod conflict;
pub mod manager;
pub mod commands;
pub mod agent_integration;

// Re-export main types for convenience
#[allow(unused_imports)]
pub use models::{
    Skill, SkillManifest, SkillStatus, SkillSelectionResult, SelectedSkill,
    SkillContext, Dependency, ConflictResolution, SkillsConfig,
};
#[allow(unused_imports)]
pub use manager::SkillsManager;
#[allow(unused_imports)]
pub use commands::{
    init_skills_manager, get_skills, discover_skills, refresh_skills,
    select_skills, get_skill, enable_skill, disable_skill,
    set_repository_url, get_skills_config, get_skill_count,
};

#[allow(unused_imports)]
use self::models::*;
#[allow(unused_imports)]
use self::manager::SkillsManager as _;
#[allow(unused_imports)]
use self::commands::*;

/// Initialize the skills system
///
/// Should be called once during application startup to set up the global
/// SkillsManager and prepare the skills system for use.
#[allow(dead_code)]
pub async fn initialize() -> Result<(), String> {
    tracing::info!("Initializing skills system");
    commands::init_skills_manager().await?;
    tracing::info!("Skills system initialized successfully");
    Ok(())
}
