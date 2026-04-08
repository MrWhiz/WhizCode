# Skills Selection Algorithm Bugfix Design

## Overview

The skills selection algorithm currently fails to select any skills from the available 235 auto-discovered skills when processing user tasks. The root cause is a combination of three factors: (1) overly conservative scoring functions that use exact matching instead of fuzzy/semantic matching, (2) a confidence threshold (0.5 or 50%) that is too high for the current scoring algorithm, and (3) poor handling of missing context (returning 0 when project type is unknown). This design formalizes the bug condition and outlines a targeted fix that improves scoring accuracy while maintaining preservation of existing behavior for niche queries and edge cases.

## Glossary

- **Bug_Condition (C)**: The condition that triggers the bug - when a user executes a task with common development terms but the system returns 0 selected skills despite relevant skills being available
- **Property (P)**: The desired behavior when the bug condition holds - the system SHALL select relevant skills with scores exceeding the confidence threshold
- **Preservation**: Existing behavior for niche queries and edge cases that must remain unchanged by the fix
- **SkillSelector**: The struct in `src-tauri/src/commands/skills/selector.rs` that implements the skills selection algorithm
- **calculate_keyword_score()**: Function that scores keyword matching between query and skill capabilities/description
- **calculate_capability_score()**: Function that scores capability alignment between query intent and skill capabilities
- **calculate_context_score()**: Function that scores workspace context fit between project type and skill requirements
- **confidence_threshold**: The minimum score (currently 0.5) required for a skill to be selected
- **SkillsConfig**: Configuration struct in `src-tauri/src/commands/skills/models.rs` that defines the confidence_threshold

## Bug Details

### Bug Condition

The bug manifests when a user executes a task containing common development terms (e.g., "create a travel vlog website using react") and the system fails to select any skills despite 235 skills being available. The `SkillSelector::select_skills()` function is either not correctly scoring skills due to overly conservative matching, not providing reasonable defaults when context is missing, or using a threshold that is too high for the current scoring algorithm.

**Formal Specification:**

```
FUNCTION isBugCondition(input)
  INPUT: input of type TaskQuery (contains task description and optional context)
  OUTPUT: boolean

  RETURN input.description CONTAINS common_development_terms
         AND available_skills_count > 0
         AND selected_skills_count == 0
         AND NOT (input.description IS empty OR all_skills_are_disabled)
END FUNCTION
```

### Examples

**Example 1: React Web Development Query**

- Input: "create a travel vlog website using react"
- Available Skills: 235 (including React, Web Development, UI/UX, etc.)
- Current Behavior: Selected 0 skills
- Expected Behavior: Select React, Web Development, UI/UX, and related skills
- Root Cause: Keyword matching uses exact substring matching; "react" matches but other keywords like "website", "vlog", "travel" don't match exactly, resulting in low keyword scores

**Example 2: Node.js Backend Query**

- Input: "build a rest api with node.js and express"
- Available Skills: 235 (including Node.js, Express, REST API, Backend, etc.)
- Current Behavior: Selected 0 skills
- Expected Behavior: Select Node.js, Express, REST API, and Backend skills
- Root Cause: Capability matching requires exact substring match; "rest-api" capability doesn't match "rest api" query intent

**Example 3: Missing Project Type Context**

- Input: "add authentication to my app"
- Available Skills: 235 (including Auth, Security, etc.)
- Current Behavior: Selected 0 skills (context_score returns 0 when project_type is empty)
- Expected Behavior: Select authentication and security skills based on keyword/capability matching alone
- Root Cause: calculate_context_score() returns 0 when project_type is empty, dragging down overall score

**Example 4: Niche Query (Preservation)**

- Input: "implement a custom blockchain consensus algorithm"
- Available Skills: 235 (mostly web/app development, few blockchain skills)
- Current Behavior: Selected 0 skills (correct - no relevant skills available)
- Expected Behavior: Continue to return 0 skills (no regression)
- Root Cause: N/A - this is correct behavior that must be preserved

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**

- When a user executes a task with no relevant skills available (e.g., niche requirements), the system SHALL CONTINUE TO return 0 selected skills without crashing
- When a user executes a task with very specific or niche requirements, the system SHALL CONTINUE TO select only highly relevant skills if they exist
- When the skill selection algorithm processes multiple tasks sequentially, the system SHALL CONTINUE TO maintain consistent scoring across all tasks
- When skills are selected and used by the agent, the system SHALL CONTINUE TO enhance the solution quality as intended by the skill integration feature
- The skill conflict detection and resolution logic SHALL CONTINUE TO work as before
- The skill indexing and lookup mechanisms SHALL CONTINUE TO function correctly

**Scope:**
All inputs that do NOT involve common development terms or that have no relevant skills should be completely unaffected by this fix. This includes:

- Queries with no matching skills (should still return 0)
- Queries with very specific/niche requirements (should still select only highly relevant skills)
- Disabled skills (should still be excluded)
- Skill conflict resolution (should still work as before)

## Hypothesized Root Cause

Based on the bug description and code analysis, the most likely issues are:

1. **Overly Conservative Keyword Matching**: The `calculate_keyword_score()` function uses exact substring matching (`contains()`), which is too strict. Keywords like "website", "vlog", "travel" don't match skill capabilities exactly, resulting in very low keyword scores even when semantically related.

2. **Exact Capability Matching**: The `calculate_capability_score()` function requires exact substring matches between query intent and capability names. For example, "rest api" doesn't match "rest-api" capability, resulting in 0 matches.

3. **Poor Context Handling**: The `calculate_context_score()` function returns 0 when project_type is empty or doesn't match any requirements, even though other scoring factors might be high. This drags down the overall score unnecessarily.

4. **Threshold Too High**: The confidence_threshold of 0.5 (50%) is too high for the current conservative scoring algorithm. With exact matching, most skills score well below 0.5, causing all skills to be filtered out.

5. **Insufficient Weighting of Keyword Matches**: When keywords do match, they contribute only 40% to the final score. If keyword_score is 0.5 and other scores are low, the final score might be: (0.5 × 0.40) + (0.0 × 0.35) + (0.0 × 0.25) = 0.20, well below the 0.5 threshold.

## Correctness Properties

Property 1: Bug Condition - Common Development Queries Select Skills

_For any_ task query where the bug condition holds (contains common development terms and relevant skills are available), the fixed SkillSelector SHALL select at least one relevant skill with a confidence score exceeding the adjusted threshold, and the system SHALL log the selected skills.

**Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**

Property 2: Preservation - Niche Queries and Edge Cases

_For any_ task query where the bug condition does NOT hold (no relevant skills available, niche requirements, or empty query), the fixed SkillSelector SHALL produce the same result as the original algorithm, preserving the behavior of returning 0 selected skills or selecting only highly relevant skills.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4**

## Fix Implementation

### Changes Required

Assuming our root cause analysis is correct, the following changes are required:

**File**: `src-tauri/src/commands/skills/selector.rs`

**Functions to Modify**:

1. `calculate_keyword_score()` - Implement fuzzy/partial keyword matching
2. `calculate_capability_score()` - Implement semantic similarity for capability matching
3. `calculate_context_score()` - Provide reasonable defaults when context is missing

**File**: `src-tauri/src/commands/skills/models.rs`

**Configuration to Update**:

1. `SkillsConfig::confidence_threshold` - Reduce from 0.5 to 0.3 (30%)

### Specific Changes

1. **Fuzzy Keyword Matching** (calculate_keyword_score):
   - Replace exact substring matching with fuzzy matching using Levenshtein distance or similar algorithm
   - Consider keywords as matching if they are similar enough (e.g., 80%+ similarity)
   - This allows "website" to match "web", "vlog" to match "video", etc.
   - Maintain backward compatibility: exact matches should still score highest

2. **Semantic Capability Matching** (calculate_capability_score):
   - Replace exact substring matching with semantic similarity
   - Normalize capability names (convert "rest-api" to "rest api" for comparison)
   - Use word-level matching: if query intent contains any word that matches a capability word, count it as a match
   - Example: "rest api" query should match "rest-api" capability

3. **Better Context Handling** (calculate_context_score):
   - When project_type is empty or unknown, return a neutral score (e.g., 0.5) instead of 0
   - This allows keyword and capability scores to determine skill selection when context is missing
   - When project_type is provided, use the existing matching logic

4. **Threshold Adjustment** (SkillsConfig):
   - Reduce confidence_threshold from 0.5 to 0.3 (30%)
   - This allows more skills to be selected while maintaining quality
   - Alternative: Implement dynamic threshold based on available skills count (e.g., if only 1 skill matches, lower threshold)

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate the bug on unfixed code, then verify the fix works correctly and preserves existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate the bug BEFORE implementing the fix. Confirm or refute the root cause analysis. If we refute, we will need to re-hypothesize.

**Test Plan**: Write tests that simulate common development queries and assert that the SkillSelector returns at least one skill. Run these tests on the UNFIXED code to observe failures and understand the root cause.

**Test Cases**:

1. **React Web Development Test**: Query "create a travel vlog website using react" with 235 available skills (will fail on unfixed code - returns 0 skills)
2. **Node.js Backend Test**: Query "build a rest api with node.js and express" with 235 available skills (will fail on unfixed code - returns 0 skills)
3. **Missing Context Test**: Query "add authentication to my app" with empty project_type context (will fail on unfixed code - returns 0 skills)
4. **Partial Keyword Match Test**: Query "web development" where only "web" matches exactly (will fail on unfixed code - low score)

**Expected Counterexamples**:

- SkillSelector returns 0 skills for common development queries
- Keyword scores are very low (0.0-0.2) due to exact matching requirements
- Capability scores are 0 when query intent doesn't exactly match capability names
- Context scores are 0 when project_type is empty or doesn't match exactly

### Fix Checking

**Goal**: Verify that for all inputs where the bug condition holds, the fixed function produces the expected behavior.

**Pseudocode:**

```
FOR ALL query WHERE isBugCondition(query) DO
  selected_skills := fixedSkillSelector.select_skills(query)
  ASSERT selected_skills.count > 0
  ASSERT selected_skills[0].confidence_score >= adjusted_threshold
  ASSERT selected_skills contains semantically relevant skills
END FOR
```

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the fixed function produces the same result as the original function.

**Pseudocode:**

```
FOR ALL query WHERE NOT isBugCondition(query) DO
  ASSERT originalSkillSelector.select_skills(query) = fixedSkillSelector.select_skills(query)
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:

- It generates many test cases automatically across the input domain
- It catches edge cases that manual unit tests might miss
- It provides strong guarantees that behavior is unchanged for all non-buggy inputs

**Test Plan**: Observe behavior on UNFIXED code first for niche queries and edge cases, then write property-based tests capturing that behavior.

**Test Cases**:

1. **Niche Query Preservation**: Verify that queries with no relevant skills continue to return 0 skills
2. **Specific Requirement Preservation**: Verify that queries with very specific requirements continue to select only highly relevant skills
3. **Disabled Skills Preservation**: Verify that disabled skills continue to be excluded from selection
4. **Conflict Resolution Preservation**: Verify that skill conflict detection and resolution continue to work correctly
5. **Multiple Task Consistency**: Verify that scoring remains consistent across multiple sequential queries

### Unit Tests

- Test fuzzy keyword matching with various similarity thresholds
- Test semantic capability matching with normalized names
- Test context score calculation with empty, partial, and complete context
- Test threshold adjustment with different available skills counts
- Test edge cases (empty query, no skills, all skills disabled)

### Property-Based Tests

- Generate random queries with common development terms and verify at least one skill is selected
- Generate random skill configurations and verify scoring consistency
- Generate random context values and verify context score is never negative
- Test that all non-buggy queries continue to produce the same results as before

### Integration Tests

- Test full skill selection flow with real skill data
- Test skill selection with various project types and contexts
- Test that selected skills are actually used by the agent to enhance solutions
- Test that skill selection doesn't break existing agent functionality
