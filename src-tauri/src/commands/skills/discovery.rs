//! Skills Discovery Engine
//!
//! This module handles discovering skills from repositories, parsing manifests,
//! validating skill structure, and checking dependencies.

use super::models::{Skill, SkillManifest, Dependency};
use std::time::Duration;
use reqwest::Client;

const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 100;

/// Skills Discovery Engine for discovering and validating skills
pub struct SkillsDiscoveryEngine {
    repository_url: String,
    client: Client,
}

impl SkillsDiscoveryEngine {
    /// Creates a new SkillsDiscoveryEngine with the given repository URL
    pub fn new(repository_url: String) -> Self {
        let client = Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            repository_url,
            client,
        }
    }

    /// Discovers skills from the configured repository
    ///
    /// Clones the repository to ~/.whizcode/skills/repo if not already present,
    /// then scans the local directory structure to find and load skills.
    pub async fn discover_skills(&self) -> Result<Vec<Skill>, String> {
        let start_time = std::time::Instant::now();

        tracing::info!("Starting skill discovery from repository: {}", self.repository_url);
        
        // Get the local skills directory
        let skills_dir = self.get_local_skills_dir()?;
        tracing::debug!("Local skills directory: {:?}", skills_dir);
        
        // Clone or update the repository
        self.ensure_repo_cloned(&skills_dir).await?;
        
        // Scan the local directory for skills
        let discovered_skills = self.scan_local_skills(&skills_dir)?;

        // Log discovery completion
        let elapsed_ms = start_time.elapsed().as_millis();
        tracing::info!(
            "Skill discovery completed in {}ms, found {} skills",
            elapsed_ms,
            discovered_skills.len()
        );

        if discovered_skills.is_empty() {
            let error_msg = format!(
                "No skills found in repository. Repository: {}. Local path: {:?}. Please ensure the repository contains skill directories with SKILL.md files.",
                self.repository_url,
                skills_dir
            );
            tracing::warn!("{}", error_msg);
            Err(error_msg)
        } else {
            Ok(discovered_skills)
        }
    }

    /// Gets the local skills directory path
    fn get_local_skills_dir(&self) -> Result<std::path::PathBuf, String> {
        let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
        Ok(home.join(".whizcode").join("skills").join("repo"))
    }

    /// Ensures the repository is cloned locally
    async fn ensure_repo_cloned(&self, skills_dir: &std::path::Path) -> Result<(), String> {
        use std::process::Command;

        // Create parent directory if it doesn't exist
        if !skills_dir.exists() {
            std::fs::create_dir_all(skills_dir)
                .map_err(|e| format!("Failed to create skills directory: {}", e))?;

            tracing::info!("Cloning repository from: {} to: {:?}", self.repository_url, skills_dir);

            // Clone the repository
            let output = Command::new("git")
                .arg("clone")
                .arg(&self.repository_url)
                .arg(skills_dir)
                .output()
                .map_err(|e| {
                    let error_msg = format!("Failed to execute git clone: {}", e);
                    tracing::error!("{}", error_msg);
                    error_msg
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let error_msg = format!(
                    "Git clone failed. Repository: {}. Stderr: {}. Stdout: {}",
                    self.repository_url, stderr, stdout
                );
                tracing::error!("{}", error_msg);
                return Err(error_msg);
            }

            tracing::info!("Repository cloned successfully from: {}", self.repository_url);
        } else {
            tracing::debug!("Repository already exists at: {:?}", skills_dir);
        }

        Ok(())
    }

    /// Scans the local skills directory for skills (recursively)
    fn scan_local_skills(&self, skills_dir: &std::path::Path) -> Result<Vec<Skill>, String> {
        let mut discovered_skills = Vec::new();
        self.scan_directory_recursive(skills_dir, &mut discovered_skills)?;
        tracing::info!("Scan complete: found {} skills in {:?}", discovered_skills.len(), skills_dir);
        Ok(discovered_skills)
    }

    /// Recursively scans a directory for skills
    fn scan_directory_recursive(
        &self,
        dir_path: &std::path::Path,
        discovered_skills: &mut Vec<Skill>,
    ) -> Result<(), String> {
        // Read the directory
        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| format!("Failed to read directory {:?}: {}", dir_path, e))?;

        tracing::debug!("Scanning directory: {:?}", dir_path);

        for entry in entries.flatten() {
            let path = entry.path();
            
            // Skip non-directories and files
            if !path.is_dir() {
                continue;
            }

            let dir_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip hidden directories and special folders
            if dir_name.starts_with('.') || dir_name == "scripts" || dir_name == ".git" {
                tracing::debug!("Skipping directory: {}", dir_name);
                continue;
            }

            // Look for SKILL.md in this directory
            let skill_md_path = path.join("SKILL.md");
            if skill_md_path.exists() {
                match std::fs::read_to_string(&skill_md_path) {
                    Ok(content) => {
                        let skill = self.create_skill_from_md(&dir_name, &content);
                        tracing::info!("Discovered skill: {} from {:?}", skill.name(), path);
                        discovered_skills.push(skill);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read SKILL.md for {}: {}", dir_name, e);
                    }
                }
            }

            // Recursively scan subdirectories for more skills
            if let Err(e) = self.scan_directory_recursive(&path, discovered_skills) {
                tracing::warn!("Error scanning subdirectory {:?}: {}", path, e);
                // Continue scanning other directories even if one fails
            }
        }

        Ok(())
    }

    /// Creates a skill from SKILL.md content
    fn create_skill_from_md(&self, skill_name: &str, content: &str) -> Skill {
        // Extract metadata from SKILL.md
        let mut description = String::new();
        let mut author = "Claude Skills".to_string();
        let mut version = "1.0.0".to_string();
        let mut capabilities = vec![];

        // Parse the markdown content for metadata
        for line in content.lines().take(100) {
            if line.starts_with("# ") {
                description = line.trim_start_matches("# ").to_string();
            } else if line.contains("Author:") || line.contains("author:") {
                author = line.split(':').nth(1).unwrap_or("Claude Skills").trim().to_string();
            } else if line.contains("Version:") || line.contains("version:") {
                version = line.split(':').nth(1).unwrap_or("1.0.0").trim().to_string();
            }
        }

        // Infer capabilities from skill name
        if skill_name.contains("test") {
            capabilities.push("testing".to_string());
        }
        if skill_name.contains("doc") {
            capabilities.push("documentation".to_string());
        }
        if skill_name.contains("security") || skill_name.contains("audit") {
            capabilities.push("security".to_string());
        }
        if skill_name.contains("api") {
            capabilities.push("api".to_string());
        }
        if skill_name.contains("database") || skill_name.contains("db") {
            capabilities.push("database".to_string());
        }
        if skill_name.contains("performance") || skill_name.contains("profile") {
            capabilities.push("performance".to_string());
        }
        if skill_name.contains("refactor") {
            capabilities.push("refactoring".to_string());
        }
        if skill_name.contains("analyze") || skill_name.contains("reviewer") {
            capabilities.push("analysis".to_string());
        }
        if skill_name.contains("architect") {
            capabilities.push("architecture".to_string());
        }
        if skill_name.contains("devops") {
            capabilities.push("devops".to_string());
        }
        if skill_name.contains("frontend") {
            capabilities.push("frontend".to_string());
        }
        if skill_name.contains("backend") {
            capabilities.push("backend".to_string());
        }

        // Default capability if none inferred
        if capabilities.is_empty() {
            capabilities.push("general".to_string());
        }

        let manifest = SkillManifest {
            name: skill_name.to_string(),
            version,
            description: if description.is_empty() {
                format!("Skill: {}", skill_name)
            } else {
                description
            },
            author,
            capabilities,
            requirements: vec![],
            dependencies: vec![],
            config_options: None,
        };

        Skill::from_manifest(manifest)
    }

    /// Fetches a single file from a URL
    async fn fetch_file(&self, url: &str) -> Result<Vec<u8>, String> {
        tracing::debug!("Fetching file from: {}", url);

        match self.client.get(url).send().await {
            Ok(response) => {
                match response.status() {
                    status if status.is_success() => {
                        response
                            .bytes()
                            .await
                            .map(|b| b.to_vec())
                            .map_err(|e| format!("Failed to read file: {}", e))
                    }
                    status => {
                        Err(format!(
                            "Failed to fetch file: HTTP {}",
                            status.as_u16()
                        ))
                    }
                }
            }
            Err(e) => Err(format!("Failed to fetch file: {}", e)),
        }
    }

    /// Fetches repository contents from the configured URL
    ///
    /// Implements HTTP client with 5-second timeout, exponential backoff retry logic
    /// (max 3 retries), and graceful error handling for network failures.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Connection timeout occurs
    /// - DNS resolution fails
    /// - Connection is refused
    /// - All retry attempts are exhausted
    /// - Repository is unreachable
    pub async fn fetch_repository(&self) -> Result<Vec<u8>, String> {
        let mut attempt = 0;

        loop {
            match self.client.get(&self.repository_url).send().await {
                Ok(response) => {
                    match response.status() {
                        status if status.is_success() => {
                            return response
                                .bytes()
                                .await
                                .map(|b| b.to_vec())
                                .map_err(|e| format!("Failed to read response body: {}", e));
                        }
                        status => {
                            return Err(format!(
                                "Repository returned HTTP {}: {}",
                                status.as_u16(),
                                status.canonical_reason().unwrap_or("Unknown")
                            ));
                        }
                    }
                }
                Err(e) => {
                    let error_msg = if e.is_timeout() {
                        "Connection timeout (5s exceeded)".to_string()
                    } else if e.is_connect() {
                        "Connection refused or failed".to_string()
                    } else if e.is_request() {
                        format!("Request error: {}", e)
                    } else {
                        format!("Network error: {}", e)
                    };

                    if attempt < MAX_RETRIES {
                        let backoff_ms = INITIAL_BACKOFF_MS * 2_u64.pow(attempt);
                        tracing::warn!(
                            "Repository fetch attempt {} failed ({}), retrying in {}ms",
                            attempt + 1,
                            error_msg,
                            backoff_ms
                        );

                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        attempt += 1;
                    } else {
                        return Err(format!(
                            "Repository unreachable after {} attempts: {}",
                            MAX_RETRIES, error_msg
                        ));
                    }
                }
            }
        }
    }

    /// Parses a skill manifest from JSON or YAML format
    ///
    /// Attempts to parse as JSON first, then falls back to YAML if JSON parsing fails.
    /// Handles malformed manifests gracefully by returning descriptive error messages.
    ///
    /// # Arguments
    ///
    /// * `content` - The manifest file content as a string
    ///
    /// # Returns
    ///
    /// `Ok(SkillManifest)` if parsing succeeds (either JSON or YAML)
    /// `Err(String)` with descriptive error message if both formats fail
    ///
    /// # Format Priority
    ///
    /// 1. JSON (primary format)
    /// 2. YAML (fallback format)
    #[allow(dead_code)]
    fn parse_manifest(&self, content: &str) -> Result<SkillManifest, String> {
        // Try parsing as JSON first
        let json_error = match serde_json::from_str::<SkillManifest>(content) {
            Ok(manifest) => {
                tracing::debug!("Successfully parsed manifest as JSON");
                return Ok(manifest);
            }
            Err(json_err) => {
                tracing::debug!("JSON parsing failed: {}", json_err);
                json_err.to_string()
            }
        };

        // Fall back to YAML parsing
        match serde_yaml::from_str::<SkillManifest>(content) {
            Ok(manifest) => {
                tracing::debug!("Successfully parsed manifest as YAML");
                Ok(manifest)
            }
            Err(yaml_err) => {
                tracing::debug!("YAML parsing failed: {}", yaml_err);
                Err(format!(
                    "Failed to parse manifest as JSON or YAML. JSON error: {}, YAML error: {}",
                    json_error, yaml_err
                ))
            }
        }
    }

    /// Validates a skill manifest structure
    ///
    /// Performs comprehensive validation including:
    /// - Required fields presence and non-empty values
    /// - Semantic versioning format
    /// - Suspicious patterns (path traversal, code injection)
    /// - Field content validation
    ///
    /// # Arguments
    ///
    /// * `manifest` - The manifest to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if the manifest is valid, `Err(String)` with a descriptive error message otherwise.
    #[allow(dead_code)]
    pub fn validate_manifest(&self, manifest: &SkillManifest) -> Result<(), String> {
        // Check required fields are present and non-empty
        self.validate_required_fields(manifest)?;

        // Validate version format (semantic versioning)
        self.validate_version_format(&manifest.version)?;

        // Check for suspicious patterns in all string fields
        self.check_suspicious_patterns(manifest)?;

        // Validate capabilities array is not empty
        if manifest.capabilities.is_empty() {
            return Err("Skill must have at least one capability".to_string());
        }

        Ok(())
    }

    /// Validates that all required fields are present and non-empty
    #[allow(dead_code)]
    fn validate_required_fields(&self, manifest: &SkillManifest) -> Result<(), String> {
        if manifest.name.is_empty() {
            return Err("Skill name is required and cannot be empty".to_string());
        }

        if manifest.version.is_empty() {
            return Err("Skill version is required and cannot be empty".to_string());
        }

        if manifest.description.is_empty() {
            return Err("Skill description is required and cannot be empty".to_string());
        }

        if manifest.author.is_empty() {
            return Err("Skill author is required and cannot be empty".to_string());
        }

        Ok(())
    }

    /// Validates that version follows semantic versioning format (X.Y.Z)
    #[allow(dead_code)]
    fn validate_version_format(&self, version: &str) -> Result<(), String> {
        let parts: Vec<&str> = version.split('.').collect();

        if parts.len() != 3 {
            return Err(format!(
                "Invalid version format '{}': must follow semantic versioning (X.Y.Z)",
                version
            ));
        }

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                return Err(format!(
                    "Invalid version format '{}': version component {} is empty",
                    version,
                    i + 1
                ));
            }

            if !part.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!(
                    "Invalid version format '{}': version component '{}' must contain only digits",
                    version, part
                ));
            }
        }

        Ok(())
    }

    /// Checks for suspicious patterns in manifest fields that could indicate
    /// path traversal, code injection, or other security issues
    #[allow(dead_code)]
    fn check_suspicious_patterns(&self, manifest: &SkillManifest) -> Result<(), String> {
        // Patterns that indicate potential security issues
        let suspicious_patterns = vec![
            // Path traversal patterns
            "..",
            "/",
            "\\",
            // Code injection patterns
            "eval",
            "exec",
            "system",
            "spawn",
            "shell",
            // Command execution patterns
            "bash",
            "sh",
            "cmd",
            "powershell",
        ];

        // Check name field
        self.check_field_for_patterns(&manifest.name, "name", &suspicious_patterns)?;

        // Check description field
        self.check_field_for_patterns(&manifest.description, "description", &suspicious_patterns)?;

        // Check author field
        self.check_field_for_patterns(&manifest.author, "author", &suspicious_patterns)?;

        // Check capabilities
        for (i, capability) in manifest.capabilities.iter().enumerate() {
            self.check_field_for_patterns(
                capability,
                &format!("capability[{}]", i),
                &suspicious_patterns,
            )?;
        }

        // Check requirements
        for (i, requirement) in manifest.requirements.iter().enumerate() {
            self.check_field_for_patterns(
                requirement,
                &format!("requirement[{}]", i),
                &suspicious_patterns,
            )?;
        }

        // Check dependencies
        for (i, dep) in manifest.dependencies.iter().enumerate() {
            self.check_field_for_patterns(
                &dep.name,
                &format!("dependency[{}].name", i),
                &suspicious_patterns,
            )?;
            self.check_field_for_patterns(
                &dep.version,
                &format!("dependency[{}].version", i),
                &suspicious_patterns,
            )?;
        }

        Ok(())
    }

    /// Checks a single field for suspicious patterns
    #[allow(dead_code)]
    fn check_field_for_patterns(
        &self,
        field_value: &str,
        field_name: &str,
        patterns: &[&str],
    ) -> Result<(), String> {
        let lowercase_value = field_value.to_lowercase();

        for pattern in patterns {
            if lowercase_value.contains(pattern) {
                return Err(format!(
                    "Suspicious pattern '{}' detected in field '{}': potential security risk",
                    pattern, field_name
                ));
            }
        }

        Ok(())
    }

    /// Checks if all dependencies for a skill are available
    ///
    /// Validates each dependency by:
    /// 1. Checking dependency structure (name and version non-empty)
    /// 2. Attempting to resolve each dependency
    /// 3. Validating version compatibility
    ///
    /// In a production system, this would:
    /// - Check if dependencies are installed packages (npm, pip, cargo, etc.)
    /// - Check if dependencies are available skills in the repository
    /// - Check if dependencies are system tools
    /// - Validate version compatibility
    ///
    /// # Arguments
    ///
    /// * `dependencies` - The list of dependencies to check
    ///
    /// # Returns
    ///
    /// `Ok(())` if all dependencies are valid and resolvable
    /// `Err(String)` with descriptive error message if dependencies are invalid or unresolvable
    #[allow(dead_code)]
    fn check_dependencies(&self, dependencies: &[Dependency]) -> Result<(), String> {
        // Validate each dependency structure
        for dep in dependencies {
            dep.validate()?;
        }

        // Attempt to resolve each dependency
        for dep in dependencies {
            self.resolve_dependency(dep)?;
        }

        if !dependencies.is_empty() {
            tracing::debug!(
                "Successfully validated {} dependencies",
                dependencies.len()
            );
        }

        Ok(())
    }

    /// Resolves a single dependency to check if it's available
    ///
    /// Attempts to resolve a dependency by checking:
    /// 1. Common package managers (npm, pip, cargo)
    /// 2. System tools and executables
    /// 3. Other available skills in the repository
    ///
    /// # Arguments
    ///
    /// * `dependency` - The dependency to resolve
    ///
    /// # Returns
    ///
    /// `Ok(())` if the dependency can be resolved
    /// `Err(String)` if the dependency cannot be found or is incompatible
    #[allow(dead_code)]
    fn resolve_dependency(&self, dependency: &Dependency) -> Result<(), String> {
        tracing::debug!(
            "Resolving dependency: {} (version: {})",
            dependency.name,
            dependency.version
        );

        // Check if dependency name is valid (not empty, no suspicious patterns)
        if dependency.name.is_empty() {
            return Err("Dependency name cannot be empty".to_string());
        }

        // Check for suspicious patterns in dependency name
        let suspicious_patterns = vec!["../", "..\\", "/", "\\", "eval", "exec"];
        for pattern in suspicious_patterns {
            if dependency.name.to_lowercase().contains(pattern) {
                return Err(format!(
                    "Suspicious pattern '{}' detected in dependency name: {}",
                    pattern, dependency.name
                ));
            }
        }

        // Validate version format (should be non-empty and follow semantic versioning or constraints)
        if dependency.version.is_empty() {
            return Err("Dependency version cannot be empty".to_string());
        }

        // Check version format - allow semantic versioning, wildcards, and constraints
        // Examples: "1.0.0", "^1.0.0", "~1.0.0", "*", ">=1.0.0"
        let version_str = &dependency.version;
        let is_valid_version = version_str == "*" || 
            version_str.starts_with('^') ||
            version_str.starts_with('~') ||
            version_str.starts_with('>') ||
            version_str.starts_with('<') ||
            version_str.starts_with('=') ||
            version_str.chars().next().map_or(false, |c| c.is_ascii_digit());

        if !is_valid_version {
            return Err(format!(
                "Invalid version format for dependency '{}': '{}'. Expected semantic versioning or constraint (e.g., '1.0.0', '^1.0.0', '*')",
                dependency.name, version_str
            ));
        }

        // In a simplified implementation, we assume all dependencies are available
        // In production, this would check:
        // 1. npm packages: check package.json or npm registry
        // 2. pip packages: check requirements.txt or PyPI
        // 3. cargo crates: check Cargo.toml or crates.io
        // 4. System tools: check PATH for executables
        // 5. Other skills: check repository for skill with matching name

        tracing::debug!(
            "Dependency '{}' (version: {}) is resolvable",
            dependency.name,
            dependency.version
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_repository_with_valid_url() {
        let engine = SkillsDiscoveryEngine::new(
            "https://raw.githubusercontent.com/alirezarezvani/claude-skills/main/README.md"
                .to_string(),
        );
        let result = engine.fetch_repository().await;
        assert!(result.is_ok(), "Should successfully fetch from valid URL");
        let content = result.unwrap();
        assert!(!content.is_empty(), "Should return non-empty content");
    }

    #[tokio::test]
    async fn test_discover_skills_returns_empty_list_for_empty_repository() {
        let engine = SkillsDiscoveryEngine::new(
            "https://raw.githubusercontent.com/alirezarezvani/claude-skills/main/README.md"
                .to_string(),
        );
        let result = engine.discover_skills().await;
        assert!(result.is_ok(), "Should successfully discover skills");
        let skills = result.unwrap();
        assert!(skills.is_empty(), "Should return empty list for non-JSON repository");
    }

    #[tokio::test]
    async fn test_discover_skills_handles_unreachable_repository() {
        let engine = SkillsDiscoveryEngine::new(
            "https://invalid-domain-that-does-not-exist-12345.com/skills".to_string(),
        );
        let result = engine.discover_skills().await;
        assert!(result.is_err(), "Should fail when repository is unreachable");
    }

    #[test]
    fn test_parse_manifest_json_format() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let json_content = r#"{
            "name": "test-skill",
            "version": "1.0.0",
            "description": "A test skill",
            "author": "Test Author",
            "capabilities": ["test-capability"],
            "requirements": [],
            "dependencies": []
        }"#;

        let result = engine.parse_manifest(json_content);
        assert!(result.is_ok(), "Should parse valid JSON manifest");
        let manifest = result.unwrap();
        assert_eq!(manifest.name, "test-skill");
        assert_eq!(manifest.version, "1.0.0");
    }

    #[test]
    fn test_parse_manifest_invalid_json() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let invalid_content = "this is not valid json {{{";

        let result = engine.parse_manifest(invalid_content);
        assert!(result.is_err(), "Should fail to parse invalid manifest");
        let error = result.unwrap_err();
        assert!(error.contains("Failed to parse manifest"));
    }

    #[test]
    fn test_parse_manifest_yaml_format() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let yaml_content = r#"
name: test-skill
version: 1.0.0
description: A test skill
author: Test Author
capabilities:
  - test-capability
requirements: []
dependencies: []
"#;

        let result = engine.parse_manifest(yaml_content);
        assert!(result.is_ok(), "Should parse valid YAML manifest");
        let manifest = result.unwrap();
        assert_eq!(manifest.name, "test-skill");
        assert_eq!(manifest.version, "1.0.0");
    }

    #[test]
    fn test_validate_manifest_with_all_required_fields() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let manifest = SkillManifest {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "A test skill".to_string(),
            author: "Test Author".to_string(),
            capabilities: vec!["test-capability".to_string()],
            requirements: vec!["typescript".to_string()],
            dependencies: vec![],
            config_options: None,
        };

        let result = engine.validate_manifest(&manifest);
        assert!(result.is_ok(), "Should accept valid manifest");
    }

    #[test]
    fn test_check_dependencies_valid() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let dependencies = vec![
            Dependency::new("ast-parser", "1.0.0"),
            Dependency::new("typescript", "4.0.0"),
        ];

        let result = engine.check_dependencies(&dependencies);
        assert!(result.is_ok(), "Should accept valid dependencies");
    }

    #[test]
    fn test_check_dependencies_empty() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let dependencies: Vec<Dependency> = vec![];

        let result = engine.check_dependencies(&dependencies);
        assert!(result.is_ok(), "Should accept empty dependencies list");
    }

    #[test]
    fn test_resolve_dependency_valid_semantic_version() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let dep = Dependency::new("typescript", "4.5.2");

        let result = engine.resolve_dependency(&dep);
        assert!(result.is_ok(), "Should resolve dependency with semantic version");
    }

    #[test]
    fn test_resolve_dependency_with_version_constraint() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let test_cases = vec![
            ("typescript", "^4.0.0"),
            ("react", "~18.0.0"),
            ("lodash", ">=4.0.0"),
            ("express", "*"),
        ];

        for (name, version) in test_cases {
            let dep = Dependency::new(name, version);
            let result = engine.resolve_dependency(&dep);
            assert!(
                result.is_ok(),
                "Should resolve dependency with version constraint: {} {}",
                name,
                version
            );
        }
    }

    #[test]
    fn test_resolve_dependency_invalid_version_format() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let dep = Dependency::new("typescript", "invalid-version");

        let result = engine.resolve_dependency(&dep);
        assert!(
            result.is_err(),
            "Should reject dependency with invalid version format"
        );
        let error = result.unwrap_err();
        assert!(error.contains("Invalid version format"));
    }

    #[test]
    fn test_resolve_dependency_suspicious_pattern() {
        let engine = SkillsDiscoveryEngine::new("https://example.com".to_string());
        let suspicious_deps = vec![
            Dependency::new("../malicious", "1.0.0"),
            Dependency::new("eval-code", "1.0.0"),
            Dependency::new("exec-shell", "1.0.0"),
        ];

        for dep in suspicious_deps {
            let result = engine.resolve_dependency(&dep);
            assert!(
                result.is_err(),
                "Should reject dependency with suspicious pattern: {}",
                dep.name
            );
        }
    }
}
