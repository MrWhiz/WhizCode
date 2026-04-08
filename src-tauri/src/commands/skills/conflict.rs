//! Conflict Resolution for Skills
//!
//! Handles detection and resolution of conflicts between skills,
//! particularly when skills provide overlapping capabilities or
//! conflict with existing WhizCode components.

use super::models::{Skill, ConflictResolution};
use std::collections::HashMap;

/// Conflict Registry mapping skills to conflicting components
///
/// Tracks which WhizCode components conflict with which skills,
/// enabling automatic conflict resolution when skills are activated.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConflictRegistry {
    /// Map of skill names to conflicting component names
    conflicts: HashMap<String, Vec<String>>,
}

impl ConflictRegistry {
    /// Creates a new conflict registry with predefined conflicts
    pub fn new() -> Self {
        let mut conflicts = HashMap::new();

        // Define predefined conflicts between skills and WhizCode components
        // These are skills that provide functionality that overlaps with existing components
        conflicts.insert(
            "code-analysis-skill".to_string(),
            vec!["code_intelligence".to_string()],
        );
        conflicts.insert(
            "testing-skill".to_string(),
            vec!["test_runner".to_string()],
        );
        conflicts.insert(
            "documentation-skill".to_string(),
            vec!["doc_generator".to_string()],
        );

        Self { conflicts }
    }

    /// Gets conflicts for a skill
    ///
    /// Returns a list of component names that conflict with the given skill.
    #[allow(dead_code)]
    pub fn get_conflicts(&self, skill_name: &str) -> Vec<String> {
        self.conflicts
            .get(skill_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Adds a new conflict mapping
    ///
    /// Registers a conflict between a skill and a component.
    #[allow(dead_code)]
    pub fn add_conflict(&mut self, skill_name: String, component_name: String) {
        self.conflicts
            .entry(skill_name)
            .or_insert_with(Vec::new)
            .push(component_name);
    }

    /// Removes a conflict mapping
    #[allow(dead_code)]
    pub fn remove_conflict(&mut self, skill_name: &str, component_name: &str) {
        if let Some(components) = self.conflicts.get_mut(skill_name) {
            components.retain(|c| c != component_name);
        }
    }

    /// Gets all registered conflicts
    #[allow(dead_code)]
    pub fn all_conflicts(&self) -> HashMap<String, Vec<String>> {
        self.conflicts.clone()
    }
}

impl Default for ConflictRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Component state tracking
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct ComponentState {
    /// Component name
    pub name: String,
    /// Whether the component is enabled
    pub enabled: bool,
    /// Previous state (for rollback)
    pub previous_enabled: Option<bool>,
}

impl ComponentState {
    /// Creates a new component state
    #[allow(dead_code)]
    pub fn new(name: String, enabled: bool) -> Self {
        Self {
            name,
            enabled,
            previous_enabled: None,
        }
    }

    /// Creates a component state with previous state tracking
    #[allow(dead_code)]
    pub fn with_previous(name: String, enabled: bool, previous_enabled: bool) -> Self {
        Self {
            name,
            enabled,
            previous_enabled: Some(previous_enabled),
        }
    }

    /// Toggles the component state
    #[allow(dead_code)]
    pub fn toggle(&mut self) {
        self.previous_enabled = Some(self.enabled);
        self.enabled = !self.enabled;
    }

    /// Reverts to the previous state
    #[allow(dead_code)]
    pub fn revert(&mut self) {
        if let Some(prev) = self.previous_enabled {
            self.enabled = prev;
            self.previous_enabled = None;
        }
    }
}

/// Component Registry tracking all WhizCode components
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ComponentRegistry {
    /// Map of component names to their states
    components: HashMap<String, ComponentState>,
}

impl ComponentRegistry {
    /// Creates a new component registry
    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut components = HashMap::new();

        // Register known WhizCode components
        components.insert(
            "code_intelligence".to_string(),
            ComponentState::new("code_intelligence".to_string(), true),
        );
        components.insert(
            "terminal".to_string(),
            ComponentState::new("terminal".to_string(), true),
        );
        components.insert(
            "git_integration".to_string(),
            ComponentState::new("git_integration".to_string(), true),
        );
        components.insert(
            "test_runner".to_string(),
            ComponentState::new("test_runner".to_string(), true),
        );
        components.insert(
            "doc_generator".to_string(),
            ComponentState::new("doc_generator".to_string(), true),
        );

        Self { components }
    }

    /// Gets the state of a component
    #[allow(dead_code)]
    pub fn get_component_state(&self, name: &str) -> Option<ComponentState> {
        self.components.get(name).cloned()
    }

    /// Sets the state of a component
    #[allow(dead_code)]
    pub fn set_component_state(&mut self, name: String, enabled: bool) {
        if let Some(component) = self.components.get_mut(&name) {
            component.previous_enabled = Some(component.enabled);
            component.enabled = enabled;
        } else {
            self.components
                .insert(name.clone(), ComponentState::new(name, enabled));
        }
    }

    /// Disables a component
    #[allow(dead_code)]
    pub fn disable_component(&mut self, name: &str) -> Result<(), String> {
        if let Some(component) = self.components.get_mut(name) {
            component.previous_enabled = Some(component.enabled);
            component.enabled = false;
            Ok(())
        } else {
            Err(format!("Component '{}' not found", name))
        }
    }

    /// Enables a component
    #[allow(dead_code)]
    pub fn enable_component(&mut self, name: &str) -> Result<(), String> {
        if let Some(component) = self.components.get_mut(name) {
            component.previous_enabled = Some(component.enabled);
            component.enabled = true;
            Ok(())
        } else {
            Err(format!("Component '{}' not found", name))
        }
    }

    /// Gets all components
    #[allow(dead_code)]
    pub fn all_components(&self) -> Vec<ComponentState> {
        self.components.values().cloned().collect()
    }

    /// Gets enabled components
    #[allow(dead_code)]
    pub fn enabled_components(&self) -> Vec<ComponentState> {
        self.components
            .values()
            .filter(|c| c.enabled)
            .cloned()
            .collect()
    }

    /// Gets disabled components
    #[allow(dead_code)]
    pub fn disabled_components(&self) -> Vec<ComponentState> {
        self.components
            .values()
            .filter(|c| !c.enabled)
            .cloned()
            .collect()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolves conflicts between selected skills
///
/// When multiple skills are selected, this function detects and resolves
/// conflicts by disabling lower-confidence conflicting skills.
#[allow(dead_code)]
pub fn resolve_skill_conflicts(
    selected_skills: &[Skill],
    registry: &ConflictRegistry,
    component_registry: &mut ComponentRegistry,
) -> Vec<ConflictResolution> {
    let mut resolutions = Vec::new();

    // Check each skill for conflicts with components
    for skill in selected_skills {
        let conflicts = registry.get_conflicts(skill.name());
        for component in conflicts {
            // Disable the conflicting component
            if let Err(e) = component_registry.disable_component(&component) {
                tracing::warn!("Failed to disable component '{}': {}", component, e);
            } else {
                tracing::info!(
                    "Disabled component '{}' due to skill '{}'",
                    component,
                    skill.name()
                );
            }

            resolutions.push(ConflictResolution::new(
                skill.name(),
                component.clone(),
                format!("Skill '{}' conflicts with component '{}'", skill.name(), component),
                skill.name(),
            ));
        }
    }

    resolutions
}

/// Reverts conflict resolutions
///
/// Re-enables components that were disabled due to skill conflicts.
#[allow(dead_code)]
pub fn revert_conflict_resolutions(
    resolutions: &[ConflictResolution],
    component_registry: &mut ComponentRegistry,
) {
    for resolution in resolutions {
        // Re-enable the component that was disabled
        if let Err(e) = component_registry.enable_component(resolution.skill_b.as_str()) {
            tracing::warn!("Failed to re-enable component '{}': {}", resolution.skill_b, e);
        } else {
            tracing::info!("Re-enabled component '{}'", resolution.skill_b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_registry_creation() {
        let registry = ConflictRegistry::new();
        assert!(!registry.conflicts.is_empty(), "Should have predefined conflicts");
    }

    #[test]
    fn test_get_conflicts() {
        let registry = ConflictRegistry::new();
        let conflicts = registry.get_conflicts("code-analysis-skill");
        assert!(!conflicts.is_empty(), "Should return conflicts for known skill");
        assert!(conflicts.contains(&"code_intelligence".to_string()));
    }

    #[test]
    fn test_add_conflict() {
        let mut registry = ConflictRegistry::new();
        registry.add_conflict("test-skill".to_string(), "test-component".to_string());
        let conflicts = registry.get_conflicts("test-skill");
        assert!(conflicts.contains(&"test-component".to_string()));
    }

    #[test]
    fn test_remove_conflict() {
        let mut registry = ConflictRegistry::new();
        registry.add_conflict("test-skill".to_string(), "test-component".to_string());
        registry.remove_conflict("test-skill", "test-component");
        let conflicts = registry.get_conflicts("test-skill");
        assert!(!conflicts.contains(&"test-component".to_string()));
    }

    #[test]
    fn test_all_conflicts() {
        let registry = ConflictRegistry::new();
        let all = registry.all_conflicts();
        assert!(!all.is_empty(), "Should return all conflicts");
    }

    #[test]
    fn test_component_state_creation() {
        let state = ComponentState::new("test".to_string(), true);
        assert_eq!(state.name, "test");
        assert!(state.enabled);
        assert_eq!(state.previous_enabled, None);
    }

    #[test]
    fn test_component_state_toggle() {
        let mut state = ComponentState::new("test".to_string(), true);
        state.toggle();
        assert!(!state.enabled);
        assert_eq!(state.previous_enabled, Some(true));
    }

    #[test]
    fn test_component_state_revert() {
        let mut state = ComponentState::new("test".to_string(), true);
        state.toggle();
        assert!(!state.enabled);
        state.revert();
        assert!(state.enabled);
    }

    #[test]
    fn test_component_registry_creation() {
        let registry = ComponentRegistry::new();
        assert!(!registry.components.is_empty(), "Should have registered components");
    }

    #[test]
    fn test_get_component_state() {
        let registry = ComponentRegistry::new();
        let state = registry.get_component_state("code_intelligence");
        assert!(state.is_some(), "Should find registered component");
        assert!(state.unwrap().enabled, "Component should be enabled by default");
    }

    #[test]
    fn test_set_component_state() {
        let mut registry = ComponentRegistry::new();
        registry.set_component_state("code_intelligence".to_string(), false);
        let state = registry.get_component_state("code_intelligence");
        assert!(!state.unwrap().enabled, "Component should be disabled");
    }

    #[test]
    fn test_disable_component() {
        let mut registry = ComponentRegistry::new();
        let result = registry.disable_component("code_intelligence");
        assert!(result.is_ok(), "Should disable component successfully");
        let state = registry.get_component_state("code_intelligence");
        assert!(!state.unwrap().enabled, "Component should be disabled");
    }

    #[test]
    fn test_enable_component() {
        let mut registry = ComponentRegistry::new();
        registry.disable_component("code_intelligence").unwrap();
        let result = registry.enable_component("code_intelligence");
        assert!(result.is_ok(), "Should enable component successfully");
        let state = registry.get_component_state("code_intelligence");
        assert!(state.unwrap().enabled, "Component should be enabled");
    }

    #[test]
    fn test_all_components() {
        let registry = ComponentRegistry::new();
        let all = registry.all_components();
        assert!(!all.is_empty(), "Should return all components");
    }

    #[test]
    fn test_enabled_components() {
        let mut registry = ComponentRegistry::new();
        registry.disable_component("code_intelligence").unwrap();
        let enabled = registry.enabled_components();
        assert!(!enabled.iter().any(|c| c.name == "code_intelligence"));
    }

    #[test]
    fn test_disabled_components() {
        let mut registry = ComponentRegistry::new();
        registry.disable_component("code_intelligence").unwrap();
        let disabled = registry.disabled_components();
        assert!(disabled.iter().any(|c| c.name == "code_intelligence"));
    }
}
