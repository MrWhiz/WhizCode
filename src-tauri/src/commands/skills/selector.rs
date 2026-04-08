//! Skills Selector and Relevance Scoring
//!
//! This module handles intelligent skill selection based on query analysis,
//! relevance scoring, conflict detection, and resolution.

use super::models::{Skill, SkillSelectionResult, SkillContext, ConflictResolution};
use std::collections::HashMap;

/// Keyword Index mapping keywords to skills for O(1) lookups
///
/// Enables efficient keyword-based skill discovery by maintaining
/// a reverse index from keywords to the skills that have them.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct KeywordIndex {
    /// Map of keyword -> Vec<skill_name>
    index: HashMap<String, Vec<String>>,
}

impl KeywordIndex {
    /// Creates a new empty keyword index
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    /// Builds the index from a list of skills
    ///
    /// Extracts keywords from each skill's capabilities and description,
    /// then builds the reverse index for efficient lookups.
    #[allow(dead_code)]
    pub fn build(skills: &[Skill]) -> Self {
        let mut index = HashMap::new();

        for skill in skills {
            let skill_name = skill.name().to_string();

            // Index keywords from capabilities
            for capability in &skill.manifest.capabilities {
                let keywords = Self::extract_keywords_from_text(capability);
                for keyword in keywords {
                    index
                        .entry(keyword)
                        .or_insert_with(Vec::new)
                        .push(skill_name.clone());
                }
            }

            // Index keywords from description
            let description_keywords = Self::extract_keywords_from_text(&skill.manifest.description);
            for keyword in description_keywords {
                index
                    .entry(keyword)
                    .or_insert_with(Vec::new)
                    .push(skill_name.clone());
            }
        }

        // Remove duplicates from each skill list
        for skills_list in index.values_mut() {
            skills_list.sort();
            skills_list.dedup();
        }

        Self { index }
    }

    /// Looks up skills by keyword
    ///
    /// Returns all skills that have the given keyword.
    #[allow(dead_code)]
    pub fn lookup(&self, keyword: &str) -> Vec<String> {
        self.index
            .get(&keyword.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    /// Extracts keywords from text by splitting on non-alphanumeric characters
    #[allow(dead_code)]
    fn extract_keywords_from_text(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(|word| word.to_string())
            .collect()
    }
}

impl Default for KeywordIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability Index mapping capabilities to skills for O(1) lookups
///
/// Enables efficient capability-based skill discovery by maintaining
/// a reverse index from capabilities to the skills that provide them.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CapabilityIndex {
    /// Map of capability -> Vec<skill_name>
    index: HashMap<String, Vec<String>>,
}

impl CapabilityIndex {
    /// Creates a new empty capability index
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    /// Builds the index from a list of skills
    ///
    /// Maps each capability to all skills that provide it.
    #[allow(dead_code)]
    pub fn build(skills: &[Skill]) -> Self {
        let mut index = HashMap::new();

        for skill in skills {
            let skill_name = skill.name().to_string();

            for capability in &skill.manifest.capabilities {
                index
                    .entry(capability.clone())
                    .or_insert_with(Vec::new)
                    .push(skill_name.clone());
            }
        }

        // Remove duplicates from each skill list
        for skills_list in index.values_mut() {
            skills_list.sort();
            skills_list.dedup();
        }

        Self { index }
    }

    /// Looks up skills by capability
    ///
    /// Returns all skills that provide the given capability.
    #[allow(dead_code)]
    pub fn lookup(&self, capability: &str) -> Vec<String> {
        self.index
            .get(capability)
            .cloned()
            .unwrap_or_default()
    }

    /// Gets all capabilities in the index
    #[allow(dead_code)]
    pub fn all_capabilities(&self) -> Vec<String> {
        self.index.keys().cloned().collect()
    }
}

impl Default for CapabilityIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Skills Selector for intelligent skill selection and scoring
#[allow(dead_code)]
pub struct SkillSelector {
    max_skills: usize,
    confidence_threshold: f32,
    keyword_index: KeywordIndex,
    capability_index: CapabilityIndex,
}

impl SkillSelector {
    /// Creates a new SkillSelector with configuration
    pub fn new(max_skills: usize, confidence_threshold: f32) -> Self {
        Self {
            max_skills,
            confidence_threshold,
            keyword_index: KeywordIndex::new(),
            capability_index: CapabilityIndex::new(),
        }
    }

    /// Creates a new SkillSelector with pre-built indices
    ///
    /// This is more efficient when you have a known set of skills
    /// and want to avoid rebuilding indices on each query.
    #[allow(dead_code)]
    pub fn with_indices(
        max_skills: usize,
        confidence_threshold: f32,
        keyword_index: KeywordIndex,
        capability_index: CapabilityIndex,
    ) -> Self {
        Self {
            max_skills,
            confidence_threshold,
            keyword_index,
            capability_index,
        }
    }

    /// Builds indices from a skill list for efficient lookups
    ///
    /// Creates keyword and capability indices that enable O(1) lookups
    /// during skill selection. Indices are cached in memory for reuse.
    ///
    /// # Arguments
    ///
    /// * `skills` - List of skills to index
    ///
    /// # Returns
    ///
    /// A tuple of (KeywordIndex, CapabilityIndex)
    #[allow(dead_code)]
    pub fn build_indices(skills: &[Skill]) -> (KeywordIndex, CapabilityIndex) {
        let keyword_index = KeywordIndex::build(skills);
        let capability_index = CapabilityIndex::build(skills);
        (keyword_index, capability_index)
    }

    /// Updates the indices with a new skill list
    ///
    /// Rebuilds both keyword and capability indices from the provided skills.
    /// This should be called when the skill list changes.
    #[allow(dead_code)]
    pub fn update_indices(&mut self, skills: &[Skill]) {
        let (keyword_index, capability_index) = Self::build_indices(skills);
        self.keyword_index = keyword_index;
        self.capability_index = capability_index;
    }

    /// Selects the most relevant skills for a given query
    ///
    /// Orchestrates the full skill selection pipeline:
    /// - Query analysis and keyword extraction
    /// - Relevance scoring based on keywords, capabilities, and context
    /// - Threshold-based filtering
    /// - Top-N skill selection
    /// - Conflict detection and resolution
    pub async fn select_skills(
        &self,
        query: &str,
        context: &SkillContext,
        available_skills: &[Skill],
    ) -> Result<SkillSelectionResult, String> {
        // Score all skills based on relevance
        let scored_skills = self.score_skills(query, context, available_skills);

        // Filter by threshold
        let filtered_skills = self.filter_by_threshold(scored_skills);

        // Select top N
        let top_skills = self.select_top_n(filtered_skills);

        // Detect conflicts
        let conflicts = self.detect_conflicts(&top_skills);

        // Resolve conflicts
        let resolved_skills = self.resolve_conflicts(top_skills, conflicts.clone());

        // Convert to SelectedSkill format
        let selected_skills = resolved_skills
            .into_iter()
            .map(|(skill, confidence)| {
                super::models::SelectedSkill::new(
                    skill.name(),
                    confidence,
                    skill.manifest.capabilities.clone(),
                    context.clone(),
                )
            })
            .collect();

        Ok(SkillSelectionResult {
            selected_skills,
            conflicts_resolved: conflicts,
        })
    }

    /// Extracts keywords from a query string
    ///
    /// Performs tokenization, lowercasing, and removes common stop words.
    fn extract_keywords(&self, query: &str) -> Vec<String> {
        let stop_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "from", "is", "are", "was", "were", "be", "been",
            "being", "have", "has", "had", "do", "does", "did", "will", "would",
            "could", "should", "may", "might", "must", "can", "this", "that",
            "these", "those", "i", "you", "he", "she", "it", "we", "they",
        ];

        query
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty() && !stop_words.contains(&word))
            .map(|word| word.to_string())
            .collect()
    }

    /// Analyzes the intent of a query
    ///
    /// Detects the primary intent from query keywords and patterns.
    /// Returns the most significant keyword or a general intent category.
    fn analyze_intent(&self, query: &str) -> String {
        let keywords = self.extract_keywords(query);
        
        if keywords.is_empty() {
            return String::new();
        }

        // Return the first significant keyword as the primary intent
        // In a more sophisticated implementation, this could use NLP or pattern matching
        keywords.first().cloned().unwrap_or_default()
    }

    /// Scores skills based on relevance to the query
    ///
    /// Implements the Skill Relevance Scoring Algorithm with three components:
    /// - Keyword matching (40% weight): matches query keywords against skill capabilities/description
    /// - Capability alignment (35% weight): matches query intent against skill capabilities
    /// - Workspace context fit (25% weight): matches workspace requirements against skill requirements
    ///
    /// Returns skills sorted by confidence score in descending order.
    fn score_skills(
        &self,
        query: &str,
        context: &SkillContext,
        skills: &[Skill],
    ) -> Vec<(Skill, f32)> {
        let query_keywords = self.extract_keywords(query);
        let query_intent = self.analyze_intent(query);

        let mut scored_skills: Vec<(Skill, f32)> = skills
            .iter()
            .filter_map(|skill| {
                // Skip disabled skills
                if !skill.enabled {
                    return None;
                }

                let mut score = 0.0;

                // 1. Keyword Matching (40% weight)
                let keyword_score = self.calculate_keyword_score(&query_keywords, skill);
                score += keyword_score * 0.40;

                // 2. Capability Alignment (35% weight)
                let capability_score = self.calculate_capability_score(&query_intent, skill);
                score += capability_score * 0.35;

                // 3. Workspace Context Fit (25% weight)
                let context_score = self.calculate_context_score(context, skill);
                score += context_score * 0.25;

                // Clamp score to [0.0, 1.0]
                let final_score = score.min(1.0).max(0.0);

                // Apply confidence threshold
                if final_score >= self.confidence_threshold {
                    Some((skill.clone(), final_score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        scored_skills.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored_skills
    }

    /// Filters skills by confidence threshold
    ///
    /// Returns only skills with confidence >= threshold.
    fn filter_by_threshold(&self, scored_skills: Vec<(Skill, f32)>) -> Vec<(Skill, f32)> {
        scored_skills
            .into_iter()
            .filter(|(_, score)| *score >= self.confidence_threshold)
            .collect()
    }

    /// Selects the top N skills from scored list
    ///
    /// Returns at most max_skills skills, maintaining score ordering.
    fn select_top_n(&self, scored_skills: Vec<(Skill, f32)>) -> Vec<(Skill, f32)> {
        scored_skills
            .into_iter()
            .take(self.max_skills)
            .collect()
    }

    /// Detects conflicts between selected skills
    ///
    /// Builds a conflict matrix comparing selected skills and identifies capability overlaps.
    fn detect_conflicts(&self, skills: &[(Skill, f32)]) -> Vec<ConflictResolution> {
        let mut conflicts = Vec::new();

        for i in 0..skills.len() {
            for j in (i + 1)..skills.len() {
                let (skill_a, confidence_a) = &skills[i];
                let (skill_b, confidence_b) = &skills[j];

                // Check for capability overlap
                let overlap: Vec<String> = skill_a
                    .manifest
                    .capabilities
                    .iter()
                    .filter(|cap| skill_b.manifest.capabilities.contains(cap))
                    .cloned()
                    .collect();

                if !overlap.is_empty() {
                    let winner = if confidence_a > confidence_b {
                        skill_a.name().to_string()
                    } else {
                        skill_b.name().to_string()
                    };

                    let conflict = ConflictResolution::new(
                        skill_a.name(),
                        skill_b.name(),
                        format!("Capability overlap: {}", overlap.join(", ")),
                        winner,
                    );
                    conflicts.push(conflict);
                }
            }
        }

        conflicts
    }

    /// Resolves conflicts by keeping higher-confidence skills
    ///
    /// For each conflict, removes the lower-confidence skill.
    fn resolve_conflicts(
        &self,
        skills: Vec<(Skill, f32)>,
        conflicts: Vec<ConflictResolution>,
    ) -> Vec<(Skill, f32)> {
        let mut resolved_skills = skills;

        for conflict in conflicts {
            let loser = conflict.loser();
            resolved_skills.retain(|(skill, _)| skill.name() != loser);
        }

        resolved_skills
    }

    /// Calculates keyword matching score (40% component)
    ///
    /// Counts how many query keywords match skill capabilities or description.
    /// Score = matches / total_keywords
    fn calculate_keyword_score(&self, query_keywords: &[String], skill: &Skill) -> f32 {
        if query_keywords.is_empty() {
            return 0.0;
        }

        let mut matches = 0;
        let query_keywords_lower: Vec<String> = query_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        // Check against skill capabilities
        for keyword in &query_keywords_lower {
            for capability in &skill.manifest.capabilities {
                if capability.to_lowercase().contains(keyword) {
                    matches += 1;
                    break;
                }
            }
        }

        // Check against skill description
        let description_lower = skill.manifest.description.to_lowercase();
        for keyword in &query_keywords_lower {
            if description_lower.contains(keyword) && !skill.manifest.capabilities.iter().any(|c| c.to_lowercase().contains(keyword)) {
                matches += 1;
            }
        }

        (matches as f32) / (query_keywords.len() as f32)
    }

    /// Calculates capability alignment score (35% component)
    ///
    /// Counts how many skill capabilities match the query intent.
    /// Score = matches / total_capabilities
    fn calculate_capability_score(&self, query_intent: &str, skill: &Skill) -> f32 {
        if skill.manifest.capabilities.is_empty() || query_intent.is_empty() {
            return 0.0;
        }

        let mut matches = 0;
        let query_intent_lower = query_intent.to_lowercase();

        for capability in &skill.manifest.capabilities {
            let capability_lower = capability.to_lowercase();
            // Check if capability relates to the query intent
            if capability_lower.contains(&query_intent_lower) || query_intent_lower.contains(&capability_lower) {
                matches += 1;
            }
        }

        (matches as f32) / (skill.manifest.capabilities.len() as f32)
    }

    /// Calculates workspace context fit score (25% component)
    ///
    /// Counts how many skill requirements match workspace context (project languages/types).
    /// Score = matches / total_requirements
    fn calculate_context_score(&self, context: &SkillContext, skill: &Skill) -> f32 {
        if skill.manifest.requirements.is_empty() {
            // If skill has no requirements, it fits any context
            return 1.0;
        }

        let mut matches = 0;
        let project_type_lower = context.project_type.to_lowercase();

        for requirement in &skill.manifest.requirements {
            let requirement_lower = requirement.to_lowercase();
            // Check if requirement matches project type or is a common language/framework
            if project_type_lower.contains(&requirement_lower) || requirement_lower.contains(&project_type_lower) {
                matches += 1;
            }
        }

        (matches as f32) / (skill.manifest.requirements.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::models::SkillManifest;
    use std::path::PathBuf;

    fn create_test_skill(name: &str, capabilities: Vec<&str>, requirements: Vec<&str>) -> Skill {
        let manifest = SkillManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("Test skill: {}", name),
            author: "Test Author".to_string(),
            capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
            requirements: requirements.iter().map(|s| s.to_string()).collect(),
            dependencies: Vec::new(),
            config_options: None,
        };

        let mut skill = Skill::new(manifest, PathBuf::from("/test/path"));
        skill.enabled = true;
        skill
    }

    #[test]
    fn test_extract_keywords() {
        let selector = SkillSelector::new(5, 0.5);

        // Test basic keyword extraction
        let keywords = selector.extract_keywords("analyze code for performance issues");
        assert!(keywords.contains(&"analyze".to_string()));
        assert!(keywords.contains(&"code".to_string()));
        assert!(keywords.contains(&"performance".to_string()));
        assert!(keywords.contains(&"issues".to_string()));

        // Stop words should be removed
        assert!(!keywords.contains(&"for".to_string()));
        assert!(!keywords.contains(&"the".to_string()));

        // Test empty query
        let empty_keywords = selector.extract_keywords("");
        assert!(empty_keywords.is_empty());

        // Test query with only stop words
        let stop_words_only = selector.extract_keywords("the a an");
        assert!(stop_words_only.is_empty());
    }

    #[test]
    fn test_analyze_intent() {
        let selector = SkillSelector::new(5, 0.5);

        let intent = selector.analyze_intent("analyze code for performance");
        assert_eq!(intent, "analyze");

        let intent2 = selector.analyze_intent("test my application");
        assert_eq!(intent2, "test");

        let empty_intent = selector.analyze_intent("");
        assert_eq!(empty_intent, "");
    }

    #[test]
    fn test_calculate_keyword_score() {
        let selector = SkillSelector::new(5, 0.5);
        let skill = create_test_skill(
            "code-analyzer",
            vec!["code-analysis", "performance-analysis", "security-check"],
            vec!["typescript", "javascript"],
        );

        // Test with matching keywords
        let keywords = vec!["code".to_string(), "analysis".to_string()];
        let score = selector.calculate_keyword_score(&keywords, &skill);
        assert!(score > 0.0);
        assert!(score <= 1.0);

        // Test with no matching keywords
        let no_match_keywords = vec!["xyz".to_string(), "abc".to_string()];
        let no_match_score = selector.calculate_keyword_score(&no_match_keywords, &skill);
        assert_eq!(no_match_score, 0.0);

        // Test with empty keywords
        let empty_keywords: Vec<String> = vec![];
        let empty_score = selector.calculate_keyword_score(&empty_keywords, &skill);
        assert_eq!(empty_score, 0.0);
    }

    #[test]
    fn test_calculate_capability_score() {
        let selector = SkillSelector::new(5, 0.5);
        let skill = create_test_skill(
            "code-analyzer",
            vec!["code-analysis", "performance-analysis", "security-check"],
            vec!["typescript"],
        );

        // Test with matching intent
        let score = selector.calculate_capability_score("code-analysis", &skill);
        assert!(score > 0.0);
        assert!(score <= 1.0);

        // Test with non-matching intent
        let no_match_score = selector.calculate_capability_score("xyz", &skill);
        assert_eq!(no_match_score, 0.0);

        // Test with empty intent
        let empty_score = selector.calculate_capability_score("", &skill);
        assert_eq!(empty_score, 0.0);
    }

    #[test]
    fn test_calculate_context_score() {
        let selector = SkillSelector::new(5, 0.5);
        let skill = create_test_skill(
            "typescript-analyzer",
            vec!["code-analysis"],
            vec!["typescript", "javascript"],
        );

        let context = SkillContext::new(
            PathBuf::from("/test"),
            "analyze code".to_string(),
            "typescript".to_string(),
            vec![],
        );

        // Test with matching context
        let score = selector.calculate_context_score(&context, &skill);
        assert!(score > 0.0);
        assert!(score <= 1.0);

        // Test with non-matching context
        let context_no_match = SkillContext::new(
            PathBuf::from("/test"),
            "analyze code".to_string(),
            "python".to_string(),
            vec![],
        );
        let no_match_score = selector.calculate_context_score(&context_no_match, &skill);
        assert_eq!(no_match_score, 0.0);

        // Test with skill that has no requirements
        let skill_no_reqs = create_test_skill("generic-skill", vec!["generic"], vec![]);
        let no_req_score = selector.calculate_context_score(&context, &skill_no_reqs);
        assert_eq!(no_req_score, 1.0);
    }

    #[test]
    fn test_score_skills_returns_sorted_results() {
        let selector = SkillSelector::new(5, 0.3);

        let skill1 = create_test_skill(
            "code-analyzer",
            vec!["code-analysis", "performance-analysis"],
            vec!["typescript"],
        );
        let skill2 = create_test_skill(
            "test-runner",
            vec!["testing", "test-execution"],
            vec!["typescript"],
        );
        let skill3 = create_test_skill(
            "doc-generator",
            vec!["documentation"],
            vec!["python"],
        );

        let skills = vec![skill1, skill2, skill3];
        let context = SkillContext::new(
            PathBuf::from("/test"),
            "analyze code performance".to_string(),
            "typescript".to_string(),
            vec![],
        );

        let scored = selector.score_skills("analyze code performance", &context, &skills);

        // Should return skills sorted by score
        assert!(!scored.is_empty());
        for i in 0..scored.len() - 1 {
            assert!(scored[i].1 >= scored[i + 1].1);
        }

        // All scores should be in valid range
        for (_, score) in &scored {
            assert!(*score >= 0.0 && *score <= 1.0);
        }
    }

    #[test]
    fn test_score_skills_respects_threshold() {
        let selector = SkillSelector::new(5, 0.8);

        let skill = create_test_skill(
            "low-relevance-skill",
            vec!["xyz"],
            vec!["abc"],
        );

        let skills = vec![skill];
        let context = SkillContext::new(
            PathBuf::from("/test"),
            "analyze code".to_string(),
            "typescript".to_string(),
            vec![],
        );

        let scored = selector.score_skills("analyze code", &context, &skills);

        // Should filter out skills below threshold
        assert!(scored.is_empty());
    }

    #[test]
    fn test_score_skills_skips_disabled_skills() {
        let selector = SkillSelector::new(5, 0.3);

        let mut skill = create_test_skill(
            "code-analyzer",
            vec!["code-analysis"],
            vec!["typescript"],
        );
        skill.enabled = false;

        let skills = vec![skill];
        let context = SkillContext::new(
            PathBuf::from("/test"),
            "analyze code".to_string(),
            "typescript".to_string(),
            vec![],
        );

        let scored = selector.score_skills("analyze code", &context, &skills);

        // Should skip disabled skills
        assert!(scored.is_empty());
    }

    #[test]
    fn test_filter_by_threshold() {
        let selector = SkillSelector::new(5, 0.5);

        let skill1 = create_test_skill("skill1", vec!["cap1"], vec![]);
        let skill2 = create_test_skill("skill2", vec!["cap2"], vec![]);
        let skill3 = create_test_skill("skill3", vec!["cap3"], vec![]);

        let scored_skills = vec![
            (skill1, 0.9),
            (skill2, 0.5),
            (skill3, 0.3),
        ];

        let filtered = selector.filter_by_threshold(scored_skills);

        // Should keep skills with score >= threshold
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|(_, score)| *score >= 0.5));
    }

    #[test]
    fn test_select_top_n() {
        let selector = SkillSelector::new(2, 0.3);

        let skill1 = create_test_skill("skill1", vec!["cap1"], vec![]);
        let skill2 = create_test_skill("skill2", vec!["cap2"], vec![]);
        let skill3 = create_test_skill("skill3", vec!["cap3"], vec![]);

        let scored_skills = vec![
            (skill1, 0.9),
            (skill2, 0.7),
            (skill3, 0.5),
        ];

        let top_n = selector.select_top_n(scored_skills);

        // Should return at most max_skills
        assert_eq!(top_n.len(), 2);
        assert_eq!(top_n[0].1, 0.9);
        assert_eq!(top_n[1].1, 0.7);
    }

    #[test]
    fn test_detect_conflicts() {
        let selector = SkillSelector::new(5, 0.3);

        let skill1 = create_test_skill(
            "analyzer1",
            vec!["code-analysis", "performance-check"],
            vec![],
        );
        let skill2 = create_test_skill(
            "analyzer2",
            vec!["code-analysis", "security-check"],
            vec![],
        );
        let skill3 = create_test_skill(
            "tester",
            vec!["testing"],
            vec![],
        );

        let skills = vec![
            (skill1, 0.9),
            (skill2, 0.8),
            (skill3, 0.7),
        ];

        let conflicts = selector.detect_conflicts(&skills);

        // Should detect conflict between skill1 and skill2 (both have code-analysis)
        assert!(!conflicts.is_empty());
        assert!(conflicts.iter().any(|c| {
            (c.skill_a == "analyzer1" && c.skill_b == "analyzer2") ||
            (c.skill_a == "analyzer2" && c.skill_b == "analyzer1")
        }));
    }

    #[test]
    fn test_detect_conflicts_no_overlap() {
        let selector = SkillSelector::new(5, 0.3);

        let skill1 = create_test_skill("analyzer", vec!["code-analysis"], vec![]);
        let skill2 = create_test_skill("tester", vec!["testing"], vec![]);

        let skills = vec![
            (skill1, 0.9),
            (skill2, 0.8),
        ];

        let conflicts = selector.detect_conflicts(&skills);

        // Should not detect conflicts when there's no overlap
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_resolve_conflicts() {
        let selector = SkillSelector::new(5, 0.3);

        let skill1 = create_test_skill("analyzer1", vec!["code-analysis"], vec![]);
        let skill2 = create_test_skill("analyzer2", vec!["code-analysis"], vec![]);

        let skills = vec![
            (skill1, 0.9),
            (skill2, 0.8),
        ];

        let conflict = ConflictResolution::new(
            "analyzer1",
            "analyzer2",
            "Capability overlap".to_string(),
            "analyzer1".to_string(),
        );

        let resolved = selector.resolve_conflicts(skills, vec![conflict]);

        // Should keep only the winner (analyzer1)
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].0.name(), "analyzer1");
    }

    #[test]
    fn test_score_skills_weights_correctly() {
        let selector = SkillSelector::new(5, 0.0);

        // Create a skill with perfect matches for all three scoring components
        let skill = create_test_skill(
            "perfect-skill",
            vec!["analyze", "code", "performance"],
            vec!["typescript"],
        );

        let skills = vec![skill];
        let context = SkillContext::new(
            PathBuf::from("/test"),
            "analyze code performance".to_string(),
            "typescript".to_string(),
            vec![],
        );

        let scored = selector.score_skills("analyze code performance", &context, &skills);

        // Should have high score due to good matches across all components
        assert!(!scored.is_empty());
        let (_, score) = &scored[0];
        assert!(*score > 0.5); // Should be reasonably high
    }

    #[test]
    fn test_score_skills_performance() {
        let selector = SkillSelector::new(5, 0.3);

        // Create many skills to test performance
        let mut skills = Vec::new();
        for i in 0..50 {
            skills.push(create_test_skill(
                &format!("skill-{}", i),
                vec!["capability-1", "capability-2"],
                vec!["typescript"],
            ));
        }

        let context = SkillContext::new(
            PathBuf::from("/test"),
            "analyze code".to_string(),
            "typescript".to_string(),
            vec![],
        );

        let start = std::time::Instant::now();
        let _scored = selector.score_skills("analyze code", &context, &skills);
        let elapsed = start.elapsed();

        // Should complete in less than 200ms
        assert!(elapsed.as_millis() < 200, "Scoring took {}ms", elapsed.as_millis());
    }

    #[test]
    fn test_keyword_index_build() {
        let skill1 = create_test_skill(
            "code-analyzer",
            vec!["code-analysis", "performance-check"],
            vec!["typescript"],
        );
        let skill2 = create_test_skill(
            "test-runner",
            vec!["testing", "test-execution"],
            vec!["typescript"],
        );

        let skills = vec![skill1, skill2];
        let index = KeywordIndex::build(&skills);

        // Should index keywords from capabilities
        let code_skills = index.lookup("code");
        assert!(code_skills.contains(&"code-analyzer".to_string()));

        let test_skills = index.lookup("test");
        assert!(test_skills.contains(&"test-runner".to_string()));
    }

    #[test]
    fn test_keyword_index_lookup() {
        let skill = create_test_skill(
            "analyzer",
            vec!["code-analysis", "performance-analysis"],
            vec![],
        );

        let index = KeywordIndex::build(&[skill]);

        // Should find skills by keyword
        let results = index.lookup("code");
        assert!(!results.is_empty());
        assert!(results.contains(&"analyzer".to_string()));

        // Should return empty for non-existent keyword
        let no_results = index.lookup("xyz");
        assert!(no_results.is_empty());
    }

    #[test]
    fn test_capability_index_build() {
        let skill1 = create_test_skill(
            "analyzer",
            vec!["code-analysis", "performance-check"],
            vec![],
        );
        let skill2 = create_test_skill(
            "tester",
            vec!["testing", "code-analysis"],
            vec![],
        );

        let skills = vec![skill1, skill2];
        let index = CapabilityIndex::build(&skills);

        // Should map capabilities to skills
        let code_analysis_skills = index.lookup("code-analysis");
        assert_eq!(code_analysis_skills.len(), 2);
        assert!(code_analysis_skills.contains(&"analyzer".to_string()));
        assert!(code_analysis_skills.contains(&"tester".to_string()));
    }

    #[test]
    fn test_capability_index_lookup() {
        let skill = create_test_skill(
            "analyzer",
            vec!["code-analysis", "performance-check"],
            vec![],
        );

        let index = CapabilityIndex::build(&[skill]);

        // Should find skills by capability
        let results = index.lookup("code-analysis");
        assert!(!results.is_empty());
        assert!(results.contains(&"analyzer".to_string()));

        // Should return empty for non-existent capability
        let no_results = index.lookup("xyz");
        assert!(no_results.is_empty());
    }

    #[test]
    fn test_build_indices() {
        let skill1 = create_test_skill(
            "analyzer",
            vec!["code-analysis"],
            vec!["typescript"],
        );
        let skill2 = create_test_skill(
            "tester",
            vec!["testing"],
            vec!["typescript"],
        );

        let skills = vec![skill1, skill2];
        let (keyword_index, capability_index) = SkillSelector::build_indices(&skills);

        // Both indices should be built
        assert!(!keyword_index.index.is_empty());
        assert!(!capability_index.index.is_empty());

        // Should be able to lookup in both
        assert!(!keyword_index.lookup("code").is_empty());
        assert!(!capability_index.lookup("code-analysis").is_empty());
    }

    #[test]
    fn test_update_indices() {
        let mut selector = SkillSelector::new(5, 0.3);

        let skill1 = create_test_skill(
            "analyzer",
            vec!["code-analysis"],
            vec![],
        );
        let skill2 = create_test_skill(
            "tester",
            vec!["testing"],
            vec![],
        );

        let skills = vec![skill1, skill2];
        selector.update_indices(&skills);

        // Indices should be updated
        assert!(!selector.keyword_index.index.is_empty());
        assert!(!selector.capability_index.index.is_empty());
    }
}
