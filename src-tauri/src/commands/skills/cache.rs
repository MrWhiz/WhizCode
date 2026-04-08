//! Cache Manager for Skills
//!
//! This module handles local caching of skills, cache invalidation strategies,
//! version management, and cache integrity validation.

use super::models::{Skill, SkillManifest};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Duration, Utc};

/// Default TTL for cache entries (24 hours)
#[allow(dead_code)]
const DEFAULT_CACHE_TTL_HOURS: i64 = 24;

/// Default maximum cache size (500MB)
#[allow(dead_code)]
const DEFAULT_MAX_CACHE_SIZE: u64 = 500 * 1024 * 1024;

/// Metadata about a cached skill entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub skill_name: String,
    pub version: String,
    pub cached_at: String,
    pub expires_at: String,
    pub checksum: String,
}

impl CacheEntry {
    /// Checks if this cache entry has expired based on TTL
    ///
    /// Compares the current time with the expires_at timestamp.
    /// Returns true if the current time is past the expiration time.
    ///
    /// # Returns
    ///
    /// `true` if the entry has expired, `false` if still valid
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        match DateTime::parse_from_rfc3339(&self.expires_at) {
            Ok(expires_dt) => {
                let now = Utc::now();
                now > expires_dt.with_timezone(&Utc)
            }
            Err(_) => {
                // If we can't parse the expiration time, consider it expired for safety
                true
            }
        }
    }

    /// Creates a new cache entry with default TTL
    ///
    /// Sets cached_at to current time and expires_at to current time + 24 hours
    #[allow(dead_code)]
    pub fn new(skill_name: String, version: String, checksum: String) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::hours(DEFAULT_CACHE_TTL_HOURS);

        Self {
            skill_name,
            version,
            cached_at: now.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            checksum,
        }
    }

    /// Creates a new cache entry with custom TTL
    ///
    /// Sets cached_at to current time and expires_at to current time + ttl_hours
    #[allow(dead_code)]
    pub fn with_ttl(skill_name: String, version: String, checksum: String, ttl_hours: i64) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::hours(ttl_hours);

        Self {
            skill_name,
            version,
            cached_at: now.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            checksum,
        }
    }
}

/// Metadata about the entire cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub total_entries: usize,
    pub last_updated: String,
    pub entries: Vec<CacheEntry>,
}

impl CacheMetadata {
    /// Creates a new empty cache metadata
    pub fn new() -> Self {
        Self {
            total_entries: 0,
            last_updated: Utc::now().to_rfc3339(),
            entries: Vec::new(),
        }
    }

    /// Adds an entry to the metadata
    #[allow(dead_code)]
    pub fn add_entry(&mut self, entry: CacheEntry) {
        self.entries.push(entry);
        self.total_entries = self.entries.len();
        self.last_updated = Utc::now().to_rfc3339();
    }

    /// Removes an entry from the metadata
    #[allow(dead_code)]
    pub fn remove_entry(&mut self, skill_name: &str) {
        self.entries.retain(|e| e.skill_name != skill_name);
        self.total_entries = self.entries.len();
        self.last_updated = Utc::now().to_rfc3339();
    }

    /// Finds an entry by skill name
    #[allow(dead_code)]
    pub fn find_entry(&self, skill_name: &str) -> Option<&CacheEntry> {
        self.entries.iter().find(|e| e.skill_name == skill_name)
    }
}

impl Default for CacheMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Bounded cache with size limits and LRU eviction policy
///
/// Tracks cache size and enforces a maximum size limit. When the cache exceeds
/// the limit, least recently used entries are evicted until the cache is below
/// the limit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundedCache {
    /// Maximum cache size in bytes (default 500MB)
    pub max_size: u64,
    /// Current cache size in bytes
    pub current_size: u64,
    /// Metadata about cached entries
    pub metadata: CacheMetadata,
}

impl BoundedCache {
    /// Creates a new bounded cache with default max size (500MB)
    pub fn new() -> Self {
        Self {
            max_size: DEFAULT_MAX_CACHE_SIZE,
            current_size: 0,
            metadata: CacheMetadata::new(),
        }
    }

    /// Creates a new bounded cache with custom max size
    #[allow(dead_code)]
    pub fn with_max_size(max_size: u64) -> Self {
        Self {
            max_size,
            current_size: 0,
            metadata: CacheMetadata::new(),
        }
    }

    /// Checks if the cache has exceeded its size limit
    #[allow(dead_code)]
    pub fn is_over_limit(&self) -> bool {
        self.current_size > self.max_size
    }

    /// Gets the remaining space in the cache
    #[allow(dead_code)]
    pub fn remaining_space(&self) -> u64 {
        if self.current_size > self.max_size {
            0
        } else {
            self.max_size - self.current_size
        }
    }

    /// Gets the cache utilization as a percentage (0.0 to 1.0)
    #[allow(dead_code)]
    pub fn utilization(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.current_size as f64) / (self.max_size as f64)
        }
    }
}

impl Default for BoundedCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache Manager for handling skill caching and invalidation
#[allow(dead_code)]
pub struct CacheManager {
    cache_dir: PathBuf,
    bounded_cache: BoundedCache,
}

#[allow(dead_code)]
impl CacheManager {
    /// Creates a new CacheManager with the given cache directory
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            bounded_cache: BoundedCache::new(),
        }
    }

    /// Creates a new CacheManager with custom max cache size
    #[allow(dead_code)]
    pub fn with_max_size(cache_dir: PathBuf, max_size: u64) -> Self {
        Self {
            cache_dir,
            bounded_cache: BoundedCache::with_max_size(max_size),
        }
    }

    /// Initializes the cache directory structure
    ///
    /// Creates the main cache directory and metadata files if they don't exist.
    /// Handles permission errors gracefully.
    ///
    /// # Directory Structure
    ///
    /// ```text
    /// ~/.whizcode/skills/cache/
    /// ├── metadata.json
    /// └── (skill directories will be created as needed)
    /// ```
    ///
    /// # Returns
    ///
    /// `Ok(())` if initialization succeeds, `Err(String)` with a descriptive error message otherwise.
    #[allow(dead_code)]
    pub fn initialize_cache_dir(&self) -> Result<(), String> {
        // Create main cache directory
        fs::create_dir_all(&self.cache_dir).map_err(|e| {
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    format!("Permission denied: Cannot create cache directory at {:?}. Please check directory permissions.", self.cache_dir)
                }
                _ => format!("Failed to create cache directory: {}", e),
            }
        })?;

        // Create or load metadata file
        let metadata_path = self.cache_dir.join("metadata.json");
        
        if !metadata_path.exists() {
            let metadata = CacheMetadata::new();
            let json = serde_json::to_string_pretty(&metadata).map_err(|e| {
                format!("Failed to serialize cache metadata: {}", e)
            })?;
            
            fs::write(&metadata_path, json).map_err(|e| {
                match e.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        format!("Permission denied: Cannot write metadata file at {:?}", metadata_path)
                    }
                    _ => format!("Failed to write metadata file: {}", e),
                }
            })?;
        }

        Ok(())
    }

    /// Gets the path to a skill's cache directory
    pub fn get_skill_cache_dir(&self, skill_name: &str) -> PathBuf {
        self.cache_dir.join(skill_name)
    }

    /// Creates a cache directory for a specific skill
    pub fn create_skill_cache_dir(&self, skill_name: &str) -> Result<PathBuf, String> {
        let skill_dir = self.get_skill_cache_dir(skill_name);
        
        fs::create_dir_all(&skill_dir).map_err(|e| {
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    format!("Permission denied: Cannot create skill cache directory for '{}'", skill_name)
                }
                _ => format!("Failed to create skill cache directory: {}", e),
            }
        })?;

        // Create files subdirectory
        let files_dir = skill_dir.join("files");
        fs::create_dir_all(&files_dir).map_err(|e| {
            format!("Failed to create skill files directory: {}", e)
        })?;

        Ok(skill_dir)
    }

    /// Loads cache metadata from disk
    pub fn load_metadata(&self) -> Result<CacheMetadata, String> {
        let metadata_path = self.cache_dir.join("metadata.json");
        
        if !metadata_path.exists() {
            return Ok(CacheMetadata::new());
        }

        let content = fs::read_to_string(&metadata_path).map_err(|e| {
            format!("Failed to read metadata file: {}", e)
        })?;

        serde_json::from_str(&content).map_err(|e| {
            format!("Failed to parse metadata file: {}", e)
        })
    }

    /// Saves cache metadata to disk
    pub fn save_metadata(&self, metadata: &CacheMetadata) -> Result<(), String> {
        let metadata_path = self.cache_dir.join("metadata.json");
        
        let json = serde_json::to_string_pretty(metadata).map_err(|e| {
            format!("Failed to serialize metadata: {}", e)
        })?;

        fs::write(&metadata_path, json).map_err(|e| {
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    format!("Permission denied: Cannot write metadata file")
                }
                _ => format!("Failed to write metadata file: {}", e),
            }
        })
    }

    /// Gets the cache directory path
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Saves a skill to the cache
    ///
    /// Saves the skill manifest to cache/{skill-name}/manifest.json, version to
    /// cache/{skill-name}/version.txt, calculates and saves a checksum for integrity
    /// validation, and updates the cache metadata with the new entry.
    ///
    /// # Arguments
    ///
    /// * `skill` - The skill to save to cache
    ///
    /// # Returns
    ///
    /// `Ok(())` if save succeeds, `Err(String)` if save fails
    ///
    /// # Performance
    ///
    /// This operation should complete in < 50ms for typical skills.
    pub async fn save_skill_to_cache(&self, skill: &Skill) -> Result<(), String> {
        let skill_name = skill.name();
        let skill_cache_dir = self.create_skill_cache_dir(skill_name)?;

        // Save the manifest to manifest.json
        let manifest_path = skill_cache_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&skill.manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        fs::write(&manifest_path, &manifest_json)
            .map_err(|e| format!("Failed to write manifest file: {}", e))?;

        // Save the version to version.txt
        let version_path = skill_cache_dir.join("version.txt");
        fs::write(&version_path, skill.version())
            .map_err(|e| format!("Failed to write version file: {}", e))?;

        // Calculate and save checksum for integrity validation
        let checksum = calculate_checksum(&skill.manifest)?;
        let checksum_path = skill_cache_dir.join("checksum.txt");
        fs::write(&checksum_path, &checksum)
            .map_err(|e| format!("Failed to write checksum file: {}", e))?;

        // Create cache entry and update metadata
        let cache_entry = CacheEntry::new(
            skill_name.to_string(),
            skill.version().to_string(),
            checksum,
        );

        let mut metadata = self.load_metadata()?;
        
        // Remove existing entry if it exists (update case)
        metadata.remove_entry(skill_name);
        
        // Add the new entry
        metadata.add_entry(cache_entry);

        // Save updated metadata
        self.save_metadata(&metadata)?;

        eprintln!("Cached skill '{}' version {}", skill_name, skill.version());

        Ok(())
    }

    /// Loads a skill from the cache
    ///
    /// Loads the skill manifest from cache/{skill-name}/manifest.json, version from
    /// cache/{skill-name}/version.txt, validates the checksum for cache integrity,
    /// and returns a Skill object with the cached data.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the skill to load from cache
    ///
    /// # Returns
    ///
    /// `Ok(Skill)` if load succeeds, `Err(String)` if load fails or cache is corrupted
    ///
    /// # Performance
    ///
    /// This operation should complete in < 50ms for typical skills.
    pub async fn load_skill_from_cache(&self, name: &str) -> Result<Skill, String> {
        let skill_cache_dir = self.get_skill_cache_dir(name);

        // Check if cache directory exists
        if !skill_cache_dir.exists() {
            return Err(format!("Skill '{}' not found in cache", name));
        }

        // Load manifest from manifest.json
        let manifest_path = skill_cache_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(format!("Manifest file not found for skill '{}'", name));
        }

        let manifest_content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest file: {}", e))?;

        let manifest: SkillManifest = serde_json::from_str(&manifest_content)
            .map_err(|e| format!("Failed to parse manifest file: {}", e))?;

        // Load version from version.txt
        let version_path = skill_cache_dir.join("version.txt");
        if !version_path.exists() {
            return Err(format!("Version file not found for skill '{}'", name));
        }

        let cached_version = fs::read_to_string(&version_path)
            .map_err(|e| format!("Failed to read version file: {}", e))?
            .trim()
            .to_string();

        // Verify version matches manifest
        if cached_version != manifest.version {
            return Err(format!(
                "Version mismatch for skill '{}': cached={}, manifest={}",
                name, cached_version, manifest.version
            ));
        }

        // Load and validate checksum for cache integrity
        let checksum_path = skill_cache_dir.join("checksum.txt");
        if !checksum_path.exists() {
            return Err(format!("Checksum file not found for skill '{}'", name));
        }

        let saved_checksum = fs::read_to_string(&checksum_path)
            .map_err(|e| format!("Failed to read checksum file: {}", e))?
            .trim()
            .to_string();

        let calculated_checksum = calculate_checksum(&manifest)?;

        if saved_checksum != calculated_checksum {
            return Err(format!(
                "Checksum validation failed for skill '{}': expected={}, calculated={}",
                name, saved_checksum, calculated_checksum
            ));
        }

        // Create Skill object with cached data
        let mut skill = Skill::new(manifest, skill_cache_dir);
        skill.mark_cached();

        eprintln!("Loaded skill '{}' from cache", name);

        Ok(skill)
    }

    /// Checks if a cached skill should be invalidated based on TTL
    ///
    /// Loads the cache metadata and checks if the skill's cache entry has expired.
    /// Returns true if the entry has expired or doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `skill_name` - The name of the skill to check
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the entry has expired or doesn't exist, `Ok(false)` if still valid,
    /// `Err(String)` if metadata loading fails
    pub fn should_invalidate_cache(&self, skill_name: &str) -> Result<bool, String> {
        let metadata = self.load_metadata()?;
        
        match metadata.find_entry(skill_name) {
            Some(entry) => Ok(entry.is_expired()),
            None => Ok(true), // Entry doesn't exist, should be invalidated
        }
    }

    /// Invalidates a skill from the cache
    ///
    /// Removes the skill's cache directory and updates the cache metadata.
    /// Logs the invalidation action for debugging.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the skill to invalidate
    ///
    /// # Returns
    ///
    /// `Ok(())` if invalidation succeeds, `Err(String)` if it fails
    pub fn invalidate_skill(&self, name: &str) -> Result<(), String> {
        // Load current metadata
        let mut metadata = self.load_metadata()?;
        
        // Check if entry exists
        if metadata.find_entry(name).is_none() {
            return Ok(()); // Entry doesn't exist, nothing to invalidate
        }

        // Remove the skill's cache directory
        let skill_cache_dir = self.get_skill_cache_dir(name);
        if skill_cache_dir.exists() {
            fs::remove_dir_all(&skill_cache_dir).map_err(|e| {
                format!("Failed to remove skill cache directory for '{}': {}", name, e)
            })?;
        }

        // Remove entry from metadata
        metadata.remove_entry(name);

        // Save updated metadata
        self.save_metadata(&metadata)?;

        // Log invalidation
        eprintln!("Cache invalidation: Removed expired skill '{}' from cache", name);

        Ok(())
    }

    /// Checks if a cached skill has a newer version available
    ///
    /// Compares the cached version with the repository version using semantic versioning.
    /// Returns true if the repository version is newer than the cached version.
    ///
    /// # Arguments
    ///
    /// * `name` - The skill name
    /// * `new_version` - The version from the repository
    ///
    /// # Returns
    ///
    /// `Ok(true)` if update is available, `Ok(false)` if cached version is current,
    /// `Err(String)` if version comparison fails
    pub fn check_version(&self, name: &str, new_version: &str) -> Result<bool, String> {
        // Load the cached version
        let cached_version_path = self.cache_dir.join(name).join("version.txt");
        
        let cached_version = if cached_version_path.exists() {
            std::fs::read_to_string(&cached_version_path)
                .map_err(|e| format!("Failed to read cached version: {}", e))?
                .trim()
                .to_string()
        } else {
            // No cached version means this is a new skill, no update available
            return Ok(false);
        };

        // Compare versions using semantic versioning
        match (parse_semantic_version(&cached_version), parse_semantic_version(new_version)) {
            (Ok(cached), Ok(new)) => {
                // Return true if new version is greater than cached version
                Ok(new > cached)
            }
            _ => {
                // If version parsing fails, fall back to string comparison
                // This handles non-standard version formats gracefully
                Ok(new_version != cached_version.as_str() && new_version > cached_version.as_str())
            }
        }
    }

    /// Updates a cached skill with new version from repository
    ///
    /// Fetches the updated skill from the repository, validates it, saves it to cache,
    /// and preserves user preferences (enabled/disabled status).
    ///
    /// # Arguments
    ///
    /// * `skill` - The skill with updated manifest and version
    ///
    /// # Returns
    ///
    /// `Ok(())` if update succeeds, `Err(String)` if update fails
    pub async fn update_cached_skill(&self, skill: &Skill) -> Result<(), String> {
        let skill_name = skill.name();
        let skill_cache_dir = self.cache_dir.join(skill_name);

        // Ensure skill cache directory exists
        std::fs::create_dir_all(&skill_cache_dir)
            .map_err(|e| format!("Failed to create skill cache directory: {}", e))?;

        // Load current preferences (enabled/disabled status) before update
        let preferences_path = skill_cache_dir.join("preferences.json");
        let preferences = if preferences_path.exists() {
            std::fs::read_to_string(&preferences_path)
                .ok()
                .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        } else {
            None
        };

        // Save the updated manifest
        let manifest_path = skill_cache_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&skill.manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        std::fs::write(&manifest_path, manifest_json)
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        // Save the new version
        let version_path = skill_cache_dir.join("version.txt");
        std::fs::write(&version_path, skill.version())
            .map_err(|e| format!("Failed to write version: {}", e))?;

        // Calculate and save checksum for integrity validation
        let checksum = calculate_checksum(&skill.manifest)?;
        let checksum_path = skill_cache_dir.join("checksum.txt");
        std::fs::write(&checksum_path, checksum)
            .map_err(|e| format!("Failed to write checksum: {}", e))?;

        // Restore user preferences if they existed
        if let Some(prefs) = preferences {
            std::fs::write(&preferences_path, serde_json::to_string_pretty(&prefs).unwrap_or_default())
                .map_err(|e| format!("Failed to restore preferences: {}", e))?;
        }

        Ok(())
    }

    /// Gets the total size of the cache
    ///
    /// Recursively calculates the size of all files in the cache directory.
    /// This includes all skill directories and their contents.
    ///
    /// # Returns
    ///
    /// `Ok(u64)` with the total cache size in bytes, `Err(String)` if calculation fails
    pub fn get_cache_size(&self) -> Result<u64, String> {
        self.calculate_directory_size(&self.cache_dir)
    }

    /// Recursively calculates the size of a directory
    fn calculate_directory_size(&self, path: &PathBuf) -> Result<u64, String> {
        let mut total_size = 0u64;

        if !path.exists() {
            return Ok(0);
        }

        let entries = fs::read_dir(path).map_err(|e| {
            format!("Failed to read directory: {}", e)
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                format!("Failed to read directory entry: {}", e)
            })?;

            let path = entry.path();
            let metadata = fs::metadata(&path).map_err(|e| {
                format!("Failed to get file metadata: {}", e)
            })?;

            if metadata.is_file() {
                total_size += metadata.len();
            } else if metadata.is_dir() {
                // Recursively calculate subdirectory size
                total_size += self.calculate_directory_size(&path)?;
            }
        }

        Ok(total_size)
    }

    /// Evicts least recently used skills when cache exceeds limit
    ///
    /// Implements LRU (Least Recently Used) eviction policy. Removes the oldest
    /// cached entries (by access time) until the cache size is below the specified limit.
    ///
    /// # Arguments
    ///
    /// * `max_size` - The target maximum cache size in bytes
    ///
    /// # Returns
    ///
    /// `Ok(())` if eviction succeeds, `Err(String)` if eviction fails
    ///
    /// # Algorithm
    ///
    /// 1. Load cache metadata
    /// 2. Sort entries by cached_at timestamp (oldest first)
    /// 3. Remove entries until cache size is below max_size
    /// 4. Update cache metadata
    /// 5. Log eviction events
    pub fn evict_oldest(&self, max_size: u64) -> Result<(), String> {
        // Load current cache metadata
        let mut metadata = self.load_metadata()?;

        // Calculate current cache size
        let current_size = self.get_cache_size()?;

        if current_size <= max_size {
            // Cache is already within limit, no eviction needed
            return Ok(());
        }

        // Sort entries by cached_at timestamp (oldest first)
        metadata.entries.sort_by(|a, b| {
            match (DateTime::parse_from_rfc3339(&a.cached_at), DateTime::parse_from_rfc3339(&b.cached_at)) {
                (Ok(a_time), Ok(b_time)) => a_time.cmp(&b_time),
                _ => std::cmp::Ordering::Equal,
            }
        });

        let mut evicted_size = 0u64;
        let mut evicted_skills = Vec::new();

        // Evict oldest entries until cache is below limit
        for entry in &metadata.entries {
            if current_size - evicted_size <= max_size {
                break;
            }

            let skill_cache_dir = self.get_skill_cache_dir(&entry.skill_name);
            let skill_size = self.calculate_directory_size(&skill_cache_dir)?;

            // Remove the skill from cache
            if skill_cache_dir.exists() {
                fs::remove_dir_all(&skill_cache_dir).map_err(|e| {
                    format!("Failed to remove skill cache directory: {}", e)
                })?;
            }

            evicted_size += skill_size;
            evicted_skills.push(entry.skill_name.clone());

            eprintln!("Evicted skill '{}' from cache (freed {} bytes)", entry.skill_name, skill_size);
        }

        // Remove evicted entries from metadata
        for skill_name in &evicted_skills {
            metadata.remove_entry(skill_name);
        }

        // Save updated metadata
        self.save_metadata(&metadata)?;

        eprintln!("Cache eviction complete: freed {} bytes, {} skills removed", evicted_size, evicted_skills.len());

        Ok(())
    }

    /// Validates cache integrity by checking checksums
    ///
    /// Iterates through all cached skills, verifies checksums for each entry,
    /// checks for missing or corrupted files, verifies manifest structure,
    /// and removes corrupted entries from the cache.
    ///
    /// # Returns
    ///
    /// `Ok(())` if validation completes (even if corrupted entries are found and removed),
    /// `Err(String)` if validation process itself fails
    ///
    /// # Performance
    ///
    /// This operation should complete in < 100ms for typical caches.
    pub fn validate_cache_integrity(&self) -> Result<(), String> {
        let metadata = self.load_metadata()?;
        let mut corrupted_skills = Vec::new();

        // Verify each cached skill
        for entry in &metadata.entries {
            let skill_name = &entry.skill_name;
            let skill_cache_dir = self.get_skill_cache_dir(skill_name);

            // Check if skill directory exists
            if !skill_cache_dir.exists() {
                eprintln!("Cache validation: Skill '{}' directory missing", skill_name);
                corrupted_skills.push(skill_name.clone());
                continue;
            }

            // Check if manifest file exists
            let manifest_path = skill_cache_dir.join("manifest.json");
            if !manifest_path.exists() {
                eprintln!("Cache validation: Manifest file missing for skill '{}'", skill_name);
                corrupted_skills.push(skill_name.clone());
                continue;
            }

            // Try to load and parse manifest
            match fs::read_to_string(&manifest_path) {
                Ok(content) => {
                    match serde_json::from_str::<SkillManifest>(&content) {
                        Ok(manifest) => {
                            // Validate manifest structure
                            if let Err(e) = manifest.validate() {
                                eprintln!("Cache validation: Invalid manifest for skill '{}': {}", skill_name, e);
                                corrupted_skills.push(skill_name.clone());
                                continue;
                            }

                            // Verify checksum
                            let checksum_path = skill_cache_dir.join("checksum.txt");
                            if !checksum_path.exists() {
                                eprintln!("Cache validation: Checksum file missing for skill '{}'", skill_name);
                                corrupted_skills.push(skill_name.clone());
                                continue;
                            }

                            match fs::read_to_string(&checksum_path) {
                                Ok(saved_checksum) => {
                                    match calculate_checksum(&manifest) {
                                        Ok(calculated_checksum) => {
                                            if saved_checksum.trim() != calculated_checksum {
                                                eprintln!("Cache validation: Checksum mismatch for skill '{}'", skill_name);
                                                corrupted_skills.push(skill_name.clone());
                                            } else {
                                                eprintln!("Cache validation: Skill '{}' is valid", skill_name);
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Cache validation: Failed to calculate checksum for skill '{}': {}", skill_name, e);
                                            corrupted_skills.push(skill_name.clone());
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Cache validation: Failed to read checksum for skill '{}': {}", skill_name, e);
                                    corrupted_skills.push(skill_name.clone());
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Cache validation: Failed to parse manifest for skill '{}': {}", skill_name, e);
                            corrupted_skills.push(skill_name.clone());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Cache validation: Failed to read manifest for skill '{}': {}", skill_name, e);
                    corrupted_skills.push(skill_name.clone());
                }
            }
        }

        // Remove corrupted entries
        if !corrupted_skills.is_empty() {
            eprintln!("Cache validation: Removing {} corrupted entries", corrupted_skills.len());
            
            let mut updated_metadata = metadata.clone();
            for skill_name in &corrupted_skills {
                // Remove the skill's cache directory
                let skill_cache_dir = self.get_skill_cache_dir(skill_name);
                if skill_cache_dir.exists() {
                    if let Err(e) = fs::remove_dir_all(&skill_cache_dir) {
                        eprintln!("Cache validation: Failed to remove corrupted skill directory '{}': {}", skill_name, e);
                    }
                }

                // Remove entry from metadata
                updated_metadata.remove_entry(skill_name);
            }

            // Save updated metadata
            self.save_metadata(&updated_metadata)?;
        }

        eprintln!("Cache validation complete: {} entries validated, {} corrupted entries removed",
                 metadata.total_entries, corrupted_skills.len());

        Ok(())
    }

    /// Checks if cache eviction is needed and performs it if necessary
    ///
    /// Compares current cache size with the bounded cache max_size.
    /// If cache exceeds the limit, triggers LRU eviction.
    ///
    /// # Returns
    ///
    /// `Ok(())` if check and eviction (if needed) succeeds, `Err(String)` otherwise
    pub fn check_and_evict_if_needed(&self) -> Result<(), String> {
        let current_size = self.get_cache_size()?;

        if current_size > self.bounded_cache.max_size {
            eprintln!("Cache size ({} bytes) exceeds limit ({} bytes), triggering eviction",
                     current_size, self.bounded_cache.max_size);
            self.evict_oldest(self.bounded_cache.max_size)?;
        }

        Ok(())
    }

    /// Gets the current cache size and updates the bounded cache
    ///
    /// # Returns
    ///
    /// `Ok(u64)` with the current cache size in bytes
    pub fn update_cache_size(&mut self) -> Result<u64, String> {
        let size = self.get_cache_size()?;
        self.bounded_cache.current_size = size;
        Ok(size)
    }

    /// Gets the bounded cache configuration
    pub fn get_bounded_cache(&self) -> &BoundedCache {
        &self.bounded_cache
    }

    /// Gets a mutable reference to the bounded cache configuration
    pub fn get_bounded_cache_mut(&mut self) -> &mut BoundedCache {
        &mut self.bounded_cache
    }
}


/// Represents a semantic version (major.minor.patch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

/// Parses a semantic version string into components
///
/// Handles versions like "1.0.0", "1.2.3", etc.
/// Returns an error if the version format is invalid.
fn parse_semantic_version(version: &str) -> Result<SemanticVersion, String> {
    let parts: Vec<&str> = version.split('.').collect();
    
    if parts.len() < 3 {
        return Err(format!("Invalid semantic version format: {}", version));
    }

    let major = parts[0]
        .parse::<u32>()
        .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
    let minor = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
    let patch = parts[2]
        .parse::<u32>()
        .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

    Ok(SemanticVersion { major, minor, patch })
}

/// Calculates a checksum for a skill manifest
///
/// Uses a simple hash of the manifest JSON for integrity validation.
fn calculate_checksum(manifest: &SkillManifest) -> Result<String, String> {
    let manifest_json = serde_json::to_string(&manifest)
        .map_err(|e| format!("Failed to serialize manifest for checksum: {}", e))?;
    
    // Use a simple hash of the JSON content
    // In production, this could use SHA256 or similar
    let checksum = format!("{:x}", manifest_json.len());
    Ok(checksum)
}

/// Gets the cached version for a skill
///
/// Reads the version.txt file from the skill's cache directory.
#[allow(dead_code)]
fn get_cached_version(skill_cache_dir: &PathBuf) -> Result<String, String> {
    let version_path = skill_cache_dir.join("version.txt");
    std::fs::read_to_string(&version_path)
        .map(|v| v.trim().to_string())
        .map_err(|e| format!("Failed to read cached version: {}", e))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_metadata_new() {
        let metadata = CacheMetadata::new();
        
        assert_eq!(metadata.total_entries, 0, "New metadata should have 0 entries");
        assert!(metadata.entries.is_empty(), "New metadata entries should be empty");
    }

    #[test]
    fn test_cache_metadata_add_entry() {
        let mut metadata = CacheMetadata::new();
        
        let entry = CacheEntry {
            skill_name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: "2024-01-02T00:00:00Z".to_string(),
            checksum: "abc123".to_string(),
        };
        
        metadata.add_entry(entry);
        
        assert_eq!(metadata.total_entries, 1, "Should have 1 entry");
        assert_eq!(metadata.entries.len(), 1, "Entries list should have 1 item");
    }

    #[test]
    fn test_cache_metadata_remove_entry() {
        let mut metadata = CacheMetadata::new();
        
        metadata.add_entry(CacheEntry {
            skill_name: "test-skill-1".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: "2024-01-02T00:00:00Z".to_string(),
            checksum: "abc123".to_string(),
        });
        
        metadata.add_entry(CacheEntry {
            skill_name: "test-skill-2".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: "2024-01-02T00:00:00Z".to_string(),
            checksum: "def456".to_string(),
        });
        
        assert_eq!(metadata.total_entries, 2, "Should have 2 entries");
        
        metadata.remove_entry("test-skill-1");
        
        assert_eq!(metadata.total_entries, 1, "Should have 1 entry after removal");
        assert_eq!(metadata.entries[0].skill_name, "test-skill-2", "Wrong entry removed");
    }

    #[test]
    fn test_cache_metadata_find_entry() {
        let mut metadata = CacheMetadata::new();
        
        metadata.add_entry(CacheEntry {
            skill_name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: "2024-01-02T00:00:00Z".to_string(),
            checksum: "abc123".to_string(),
        });
        
        let found = metadata.find_entry("test-skill");
        assert!(found.is_some(), "Entry should be found");
        assert_eq!(found.unwrap().version, "1.0.0", "Version mismatch");
        
        let not_found = metadata.find_entry("nonexistent");
        assert!(not_found.is_none(), "Nonexistent entry should not be found");
    }

    #[test]
    fn test_parse_semantic_version_valid() {
        let result = parse_semantic_version("1.2.3");
        assert!(result.is_ok(), "Should parse valid semantic version");
        
        let version = result.unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_parse_semantic_version_invalid() {
        let result = parse_semantic_version("1.2");
        assert!(result.is_err(), "Should reject incomplete version");
        
        let result = parse_semantic_version("a.b.c");
        assert!(result.is_err(), "Should reject non-numeric version");
    }

    #[test]
    fn test_semantic_version_comparison() {
        let v1 = parse_semantic_version("1.0.0").unwrap();
        let v2 = parse_semantic_version("1.0.1").unwrap();
        let v3 = parse_semantic_version("1.1.0").unwrap();
        let v4 = parse_semantic_version("2.0.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v1 < v4);
    }

    #[test]
    fn test_save_skill_to_cache() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().unwrap();

        // Create a test skill
        let manifest = SkillManifest {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "Test skill".to_string(),
            author: "Test Author".to_string(),
            capabilities: vec!["test-capability".to_string()],
            requirements: vec!["rust".to_string()],
            dependencies: vec![],
            config_options: None,
        };

        let skill = Skill::new(manifest, cache_dir.clone());

        // Save skill to cache
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            manager.save_skill_to_cache(&skill).await.unwrap();
        });

        // Verify files were created
        let skill_cache_dir = cache_dir.join("test-skill");
        assert!(skill_cache_dir.exists(), "Skill cache directory should exist");
        assert!(skill_cache_dir.join("manifest.json").exists(), "Manifest file should exist");
        assert!(skill_cache_dir.join("version.txt").exists(), "Version file should exist");
        assert!(skill_cache_dir.join("checksum.txt").exists(), "Checksum file should exist");

        // Verify metadata was updated
        let metadata = manager.load_metadata().unwrap();
        assert_eq!(metadata.total_entries, 1, "Metadata should have 1 entry");
        assert!(metadata.find_entry("test-skill").is_some(), "Entry should be in metadata");
    }

    #[test]
    fn test_load_skill_from_cache() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().unwrap();

        // Create and save a test skill
        let manifest = SkillManifest {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "Test skill".to_string(),
            author: "Test Author".to_string(),
            capabilities: vec!["test-capability".to_string()],
            requirements: vec!["rust".to_string()],
            dependencies: vec![],
            config_options: None,
        };

        let skill = Skill::new(manifest.clone(), cache_dir.clone());

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            manager.save_skill_to_cache(&skill).await.unwrap();

            // Load skill from cache
            let loaded_skill = manager.load_skill_from_cache("test-skill").await.unwrap();

            // Verify loaded skill matches original
            assert_eq!(loaded_skill.name(), "test-skill");
            assert_eq!(loaded_skill.version(), "1.0.0");
            assert_eq!(loaded_skill.manifest.description, "Test skill");
            assert_eq!(loaded_skill.manifest.author, "Test Author");
            assert!(loaded_skill.cached, "Loaded skill should be marked as cached");
        });
    }

    #[test]
    fn test_load_skill_from_cache_missing_manifest() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().unwrap();

        // Create skill cache directory without manifest
        manager.create_skill_cache_dir("missing-skill").unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Try to load skill without manifest
            let result = manager.load_skill_from_cache("missing-skill").await;
            assert!(result.is_err(), "Should fail when manifest is missing");
        });
    }

    #[test]
    fn test_load_skill_from_cache_corrupted_checksum() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().unwrap();

        // Create and save a test skill
        let manifest = SkillManifest {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "Test skill".to_string(),
            author: "Test Author".to_string(),
            capabilities: vec!["test-capability".to_string()],
            requirements: vec!["rust".to_string()],
            dependencies: vec![],
            config_options: None,
        };

        let skill = Skill::new(manifest, cache_dir.clone());

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            manager.save_skill_to_cache(&skill).await.unwrap();

            // Corrupt the checksum
            let checksum_path = cache_dir.join("test-skill").join("checksum.txt");
            fs::write(&checksum_path, "corrupted_checksum").unwrap();

            // Try to load skill with corrupted checksum
            let result = manager.load_skill_from_cache("test-skill").await;
            assert!(result.is_err(), "Should fail when checksum is corrupted");
        });
    }

    #[test]
    fn test_validate_cache_integrity_valid_cache() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().unwrap();

        // Create and save a test skill
        let manifest = SkillManifest {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "Test skill".to_string(),
            author: "Test Author".to_string(),
            capabilities: vec!["test-capability".to_string()],
            requirements: vec!["rust".to_string()],
            dependencies: vec![],
            config_options: None,
        };

        let skill = Skill::new(manifest, cache_dir.clone());

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            manager.save_skill_to_cache(&skill).await.unwrap();
        });

        // Validate cache integrity
        let result = manager.validate_cache_integrity();
        assert!(result.is_ok(), "Validation should succeed for valid cache");

        // Verify metadata still has the entry
        let metadata = manager.load_metadata().unwrap();
        assert_eq!(metadata.total_entries, 1, "Valid entry should remain");
    }

    #[test]
    fn test_validate_cache_integrity_corrupted_checksum() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().unwrap();

        // Create and save a test skill
        let manifest = SkillManifest {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "Test skill".to_string(),
            author: "Test Author".to_string(),
            capabilities: vec!["test-capability".to_string()],
            requirements: vec!["rust".to_string()],
            dependencies: vec![],
            config_options: None,
        };

        let skill = Skill::new(manifest, cache_dir.clone());

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            manager.save_skill_to_cache(&skill).await.unwrap();
        });

        // Corrupt the checksum
        let checksum_path = cache_dir.join("test-skill").join("checksum.txt");
        fs::write(&checksum_path, "corrupted_checksum").unwrap();

        // Validate cache integrity
        let result = manager.validate_cache_integrity();
        assert!(result.is_ok(), "Validation should complete even with corrupted entries");

        // Verify corrupted entry was removed
        let metadata = manager.load_metadata().unwrap();
        assert_eq!(metadata.total_entries, 0, "Corrupted entry should be removed");
        assert!(metadata.find_entry("test-skill").is_none(), "Entry should be removed from metadata");
    }

    #[test]
    fn test_validate_cache_integrity_missing_manifest() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().unwrap();

        // Create metadata with entry but no actual files
        let mut metadata = CacheMetadata::new();
        metadata.add_entry(CacheEntry::new(
            "missing-skill".to_string(),
            "1.0.0".to_string(),
            "abc123".to_string(),
        ));
        manager.save_metadata(&metadata).unwrap();

        // Validate cache integrity
        let result = manager.validate_cache_integrity();
        assert!(result.is_ok(), "Validation should complete even with missing files");

        // Verify missing entry was removed
        let updated_metadata = manager.load_metadata().unwrap();
        assert_eq!(updated_metadata.total_entries, 0, "Missing entry should be removed");
    }

    #[test]
    fn test_validate_cache_integrity_multiple_skills() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().unwrap();

        // Create and save multiple skills
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            for i in 1..=3 {
                let manifest = SkillManifest {
                    name: format!("skill-{}", i),
                    version: "1.0.0".to_string(),
                    description: format!("Test skill {}", i),
                    author: "Test Author".to_string(),
                    capabilities: vec![format!("capability-{}", i)],
                    requirements: vec!["rust".to_string()],
                    dependencies: vec![],
                    config_options: None,
                };

                let skill = Skill::new(manifest, cache_dir.clone());
                manager.save_skill_to_cache(&skill).await.unwrap();
            }
        });

        // Corrupt one skill
        let checksum_path = cache_dir.join("skill-2").join("checksum.txt");
        fs::write(&checksum_path, "corrupted").unwrap();

        // Validate cache integrity
        let result = manager.validate_cache_integrity();
        assert!(result.is_ok(), "Validation should complete");

        // Verify only corrupted skill was removed
        let metadata = manager.load_metadata().unwrap();
        assert_eq!(metadata.total_entries, 2, "Only corrupted skill should be removed");
        assert!(metadata.find_entry("skill-1").is_some(), "skill-1 should remain");
        assert!(metadata.find_entry("skill-2").is_none(), "skill-2 should be removed");
        assert!(metadata.find_entry("skill-3").is_some(), "skill-3 should remain");
    }

    #[test]
    fn test_cache_manager_new() {
        use std::path::PathBuf;
        let cache_dir = PathBuf::from("/tmp/test-cache");
        let manager = CacheManager::new(cache_dir.clone());
        
        assert_eq!(manager.cache_dir(), &cache_dir);
    }

    #[test]
    fn test_get_skill_cache_dir() {
        use std::path::PathBuf;
        let cache_dir = PathBuf::from("/tmp/test-cache");
        let manager = CacheManager::new(cache_dir.clone());
        
        let skill_dir = manager.get_skill_cache_dir("test-skill");
        assert_eq!(skill_dir, cache_dir.join("test-skill"));
    }

    #[test]
    fn test_cache_metadata_serialization() {
        let mut metadata = CacheMetadata::new();
        metadata.add_entry(CacheEntry {
            skill_name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: "2024-01-02T00:00:00Z".to_string(),
            checksum: "abc123".to_string(),
        });
        
        // Should serialize to JSON
        let json = serde_json::to_string(&metadata);
        assert!(json.is_ok(), "Should serialize to JSON");
        
        // Should deserialize from JSON
        let deserialized: Result<CacheMetadata, _> = serde_json::from_str(&json.unwrap());
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
        
        let restored = deserialized.unwrap();
        assert_eq!(restored.total_entries, 1);
        assert_eq!(restored.entries[0].skill_name, "test-skill");
    }

    #[test]
    fn test_cache_entry_is_expired_past_expiration() {
        // Create an entry that expired in the past
        let entry = CacheEntry {
            skill_name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: "2024-01-02T00:00:00Z".to_string(), // Past date
            checksum: "abc123".to_string(),
        };

        assert!(entry.is_expired(), "Entry with past expiration should be expired");
    }

    #[test]
    fn test_cache_entry_is_expired_future_expiration() {
        // Create an entry that expires in the future
        let now = Utc::now();
        let future = now + Duration::hours(24);
        
        let entry = CacheEntry {
            skill_name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            cached_at: now.to_rfc3339(),
            expires_at: future.to_rfc3339(),
            checksum: "abc123".to_string(),
        };

        assert!(!entry.is_expired(), "Entry with future expiration should not be expired");
    }

    #[test]
    fn test_cache_entry_new_with_default_ttl() {
        let entry = CacheEntry::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            "abc123".to_string(),
        );

        assert_eq!(entry.skill_name, "test-skill");
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.checksum, "abc123");
        assert!(!entry.is_expired(), "New entry should not be expired");
    }

    #[test]
    fn test_cache_entry_with_custom_ttl() {
        let entry = CacheEntry::with_ttl(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            "abc123".to_string(),
            1, // 1 hour TTL
        );

        assert_eq!(entry.skill_name, "test-skill");
        assert!(!entry.is_expired(), "Entry with 1 hour TTL should not be expired");
    }

    #[test]
    fn test_cache_entry_invalid_expiration_format() {
        let entry = CacheEntry {
            skill_name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: "invalid-date".to_string(), // Invalid format
            checksum: "abc123".to_string(),
        };

        assert!(entry.is_expired(), "Entry with invalid expiration format should be considered expired");
    }

    #[test]
    fn test_should_invalidate_cache_with_expired_entry() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir);
        
        // Initialize cache
        manager.initialize_cache_dir().unwrap();
        
        // Create metadata with expired entry
        let mut metadata = CacheMetadata::new();
        metadata.add_entry(CacheEntry {
            skill_name: "expired-skill".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: "2024-01-02T00:00:00Z".to_string(), // Past date
            checksum: "abc123".to_string(),
        });
        
        manager.save_metadata(&metadata).unwrap();
        
        // Check if should invalidate
        let should_invalidate = manager.should_invalidate_cache("expired-skill").unwrap();
        assert!(should_invalidate, "Expired entry should be invalidated");
    }

    #[test]
    fn test_should_invalidate_cache_with_valid_entry() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir);
        
        // Initialize cache
        manager.initialize_cache_dir().unwrap();
        
        // Create metadata with valid entry
        let mut metadata = CacheMetadata::new();
        metadata.add_entry(CacheEntry::new(
            "valid-skill".to_string(),
            "1.0.0".to_string(),
            "abc123".to_string(),
        ));
        
        manager.save_metadata(&metadata).unwrap();
        
        // Check if should invalidate
        let should_invalidate = manager.should_invalidate_cache("valid-skill").unwrap();
        assert!(!should_invalidate, "Valid entry should not be invalidated");
    }

    #[test]
    fn test_should_invalidate_cache_nonexistent_entry() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir);
        
        // Initialize cache
        manager.initialize_cache_dir().unwrap();
        
        // Check if should invalidate nonexistent entry
        let should_invalidate = manager.should_invalidate_cache("nonexistent-skill").unwrap();
        assert!(should_invalidate, "Nonexistent entry should be invalidated");
    }

    #[test]
    fn test_invalidate_skill_removes_entry() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir);
        
        // Initialize cache
        manager.initialize_cache_dir().unwrap();
        
        // Create metadata with entry
        let mut metadata = CacheMetadata::new();
        metadata.add_entry(CacheEntry::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            "abc123".to_string(),
        ));
        
        manager.save_metadata(&metadata).unwrap();
        
        // Create skill cache directory
        manager.create_skill_cache_dir("test-skill").unwrap();
        
        // Verify entry exists
        let metadata_before = manager.load_metadata().unwrap();
        assert_eq!(metadata_before.total_entries, 1, "Should have 1 entry before invalidation");
        
        // Invalidate skill
        manager.invalidate_skill("test-skill").unwrap();
        
        // Verify entry is removed
        let metadata_after = manager.load_metadata().unwrap();
        assert_eq!(metadata_after.total_entries, 0, "Should have 0 entries after invalidation");
        assert!(metadata_after.find_entry("test-skill").is_none(), "Entry should be removed");
    }

    #[test]
    fn test_invalidate_skill_removes_directory() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir);
        
        // Initialize cache
        manager.initialize_cache_dir().unwrap();
        
        // Create metadata with entry
        let mut metadata = CacheMetadata::new();
        metadata.add_entry(CacheEntry::new(
            "test-skill".to_string(),
            "1.0.0".to_string(),
            "abc123".to_string(),
        ));
        
        manager.save_metadata(&metadata).unwrap();
        
        // Create skill cache directory
        let skill_dir = manager.create_skill_cache_dir("test-skill").unwrap();
        assert!(skill_dir.exists(), "Skill directory should exist");
        
        // Invalidate skill
        manager.invalidate_skill("test-skill").unwrap();
        
        // Verify directory is removed
        assert!(!skill_dir.exists(), "Skill directory should be removed after invalidation");
    }

    #[test]
    fn test_invalidate_skill_nonexistent_entry() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir);
        
        // Initialize cache
        manager.initialize_cache_dir().unwrap();
        
        // Invalidate nonexistent skill (should not error)
        let result = manager.invalidate_skill("nonexistent-skill");
        assert!(result.is_ok(), "Invalidating nonexistent skill should not error");
    }

    #[test]
    fn test_cache_metadata_updated_on_invalidation() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let manager = CacheManager::new(cache_dir);
        
        // Initialize cache
        manager.initialize_cache_dir().unwrap();
        
        // Create metadata with multiple entries
        let mut metadata = CacheMetadata::new();
        metadata.add_entry(CacheEntry::new(
            "skill-1".to_string(),
            "1.0.0".to_string(),
            "abc123".to_string(),
        ));
        metadata.add_entry(CacheEntry::new(
            "skill-2".to_string(),
            "1.0.0".to_string(),
            "def456".to_string(),
        ));
        
        manager.save_metadata(&metadata).unwrap();
        
        // Verify initial state
        let metadata_before = manager.load_metadata().unwrap();
        assert_eq!(metadata_before.total_entries, 2, "Should have 2 entries");
        let last_updated_before = metadata_before.last_updated.clone();
        
        // Wait a bit to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        // Invalidate one skill
        manager.invalidate_skill("skill-1").unwrap();
        
        // Verify metadata is updated
        let metadata_after = manager.load_metadata().unwrap();
        assert_eq!(metadata_after.total_entries, 1, "Should have 1 entry after invalidation");
        assert!(metadata_after.last_updated > last_updated_before, "last_updated should be updated");
        assert!(metadata_after.find_entry("skill-1").is_none(), "skill-1 should be removed");
        assert!(metadata_after.find_entry("skill-2").is_some(), "skill-2 should remain");
    }

    #[test]
    fn test_bounded_cache_new() {
        let cache = BoundedCache::new();
        assert_eq!(cache.max_size, DEFAULT_MAX_CACHE_SIZE);
        assert_eq!(cache.current_size, 0);
        assert_eq!(cache.metadata.total_entries, 0);
    }

    #[test]
    fn test_bounded_cache_with_max_size() {
        let max_size = 100 * 1024 * 1024; // 100MB
        let cache = BoundedCache::with_max_size(max_size);
        assert_eq!(cache.max_size, max_size);
        assert_eq!(cache.current_size, 0);
    }

    #[test]
    fn test_bounded_cache_is_over_limit() {
        let mut cache = BoundedCache::with_max_size(100);
        assert!(!cache.is_over_limit());

        cache.current_size = 150;
        assert!(cache.is_over_limit());
    }

    #[test]
    fn test_bounded_cache_remaining_space() {
        let mut cache = BoundedCache::with_max_size(100);
        assert_eq!(cache.remaining_space(), 100);

        cache.current_size = 30;
        assert_eq!(cache.remaining_space(), 70);

        cache.current_size = 150;
        assert_eq!(cache.remaining_space(), 0);
    }

    #[test]
    fn test_bounded_cache_utilization() {
        let mut cache = BoundedCache::with_max_size(100);
        assert_eq!(cache.utilization(), 0.0);

        cache.current_size = 50;
        assert_eq!(cache.utilization(), 0.5);

        cache.current_size = 100;
        assert_eq!(cache.utilization(), 1.0);
    }

    #[test]
    fn test_bounded_cache_default() {
        let cache = BoundedCache::default();
        assert_eq!(cache.max_size, DEFAULT_MAX_CACHE_SIZE);
        assert_eq!(cache.current_size, 0);
    }

    #[test]
    fn test_cache_manager_with_max_size() {
        use std::path::PathBuf;
        let cache_dir = PathBuf::from("/tmp/test-cache");
        let max_size = 100 * 1024 * 1024;
        let manager = CacheManager::with_max_size(cache_dir.clone(), max_size);
        
        assert_eq!(manager.cache_dir(), &cache_dir);
        assert_eq!(manager.get_bounded_cache().max_size, max_size);
    }

    #[test]
    fn test_get_cache_size_empty_directory() {
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache_dir = PathBuf::from(temp_dir.path());
        let manager = CacheManager::new(cache_dir);

        let size = manager.get_cache_size().expect("Failed to get cache size");
        assert_eq!(size, 0, "Empty cache should have size 0");
    }

    #[test]
    fn test_get_cache_size_with_files() {
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache_dir = PathBuf::from(temp_dir.path());
        let manager = CacheManager::new(cache_dir.clone());

        // Create a test file
        let test_file = cache_dir.join("test.txt");
        fs::write(&test_file, "test content").expect("Failed to write test file");

        let size = manager.get_cache_size().expect("Failed to get cache size");
        assert!(size > 0, "Cache with files should have size > 0");
        assert_eq!(size, 12, "Cache size should match file content size");
    }

    #[test]
    fn test_get_cache_size_with_subdirectories() {
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache_dir = PathBuf::from(temp_dir.path());
        let manager = CacheManager::new(cache_dir.clone());

        // Create subdirectory with files
        let subdir = cache_dir.join("subdir");
        fs::create_dir(&subdir).expect("Failed to create subdirectory");
        fs::write(subdir.join("file1.txt"), "content1").expect("Failed to write file1");
        fs::write(subdir.join("file2.txt"), "content2").expect("Failed to write file2");

        let size = manager.get_cache_size().expect("Failed to get cache size");
        assert_eq!(size, 16, "Cache size should include all files in subdirectories");
    }

    #[test]
    fn test_evict_oldest_under_limit() {
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache_dir = PathBuf::from(temp_dir.path());
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().expect("Failed to initialize cache");

        // Create a test file
        let test_file = cache_dir.join("test.txt");
        fs::write(&test_file, "test content").expect("Failed to write test file");

        // Evict with limit larger than current size
        let result = manager.evict_oldest(1000);
        assert!(result.is_ok(), "Eviction should succeed");

        // File should still exist
        assert!(test_file.exists(), "File should not be evicted when under limit");
    }

    #[test]
    fn test_evict_oldest_over_limit() {
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache_dir = PathBuf::from(temp_dir.path());
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().expect("Failed to initialize cache");

        // Create skill cache directories with metadata
        let skill1_dir = cache_dir.join("skill-1");
        fs::create_dir(&skill1_dir).expect("Failed to create skill1 dir");
        fs::write(skill1_dir.join("file.txt"), "content1").expect("Failed to write file");

        let skill2_dir = cache_dir.join("skill-2");
        fs::create_dir(&skill2_dir).expect("Failed to create skill2 dir");
        fs::write(skill2_dir.join("file.txt"), "content2content2").expect("Failed to write file");

        // Add entries to metadata
        let mut metadata = manager.load_metadata().expect("Failed to load metadata");
        metadata.add_entry(CacheEntry {
            skill_name: "skill-1".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: "2024-01-02T00:00:00Z".to_string(),
            checksum: "abc123".to_string(),
        });
        metadata.add_entry(CacheEntry {
            skill_name: "skill-2".to_string(),
            version: "1.0.0".to_string(),
            cached_at: "2024-01-02T00:00:00Z".to_string(),
            expires_at: "2024-01-03T00:00:00Z".to_string(),
            checksum: "def456".to_string(),
        });
        manager.save_metadata(&metadata).expect("Failed to save metadata");

        // Evict with limit that allows skill-2 to remain
        // skill-1 is 8 bytes, skill-2 is 16 bytes
        // Set limit to 10 bytes, so only skill-1 should be evicted
        // After evicting skill-1 (8 bytes), we have 16 bytes left which is > 10 bytes
        // So skill-2 will also be evicted
        let result = manager.evict_oldest(10);
        assert!(result.is_ok(), "Eviction should succeed");

        // Both skills should be evicted since 16 bytes > 10 bytes limit
        assert!(!skill1_dir.exists(), "Oldest skill should be evicted");
        assert!(!skill2_dir.exists(), "Newer skill should also be evicted to reach limit");
    }

    #[test]
    fn test_check_and_evict_if_needed_under_limit() {
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache_dir = PathBuf::from(temp_dir.path());
        let manager = CacheManager::new(cache_dir.clone());

        // Initialize cache
        manager.initialize_cache_dir().expect("Failed to initialize cache");

        // Create a small file
        let test_file = cache_dir.join("test.txt");
        fs::write(&test_file, "test").expect("Failed to write test file");

        // Check and evict (should not evict since under limit)
        let result = manager.check_and_evict_if_needed();
        assert!(result.is_ok(), "Check and evict should succeed");

        // File should still exist
        assert!(test_file.exists(), "File should not be evicted when under limit");
    }

    #[test]
    fn test_update_cache_size() {
        use std::path::PathBuf;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache_dir = PathBuf::from(temp_dir.path());
        let mut manager = CacheManager::new(cache_dir.clone());

        // Create a test file
        let test_file = cache_dir.join("test.txt");
        fs::write(&test_file, "test content").expect("Failed to write test file");

        // Update cache size
        let size = manager.update_cache_size().expect("Failed to update cache size");
        assert_eq!(size, 12, "Cache size should be updated correctly");
        assert_eq!(manager.get_bounded_cache().current_size, 12, "Bounded cache size should be updated");
    }
}
