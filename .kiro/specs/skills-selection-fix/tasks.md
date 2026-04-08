# Implementation Plan

## Phase 1: Exploration - Surface the Bug

- [ ] 1. Write bug condition exploration test
  - **Property 1: Bug Condition** - Common Development Queries Select Skills
  - **CRITICAL**: This test MUST FAIL on unfixed code - failure confirms the bug exists
  - **DO NOT attempt to fix the test or the code when it fails**
  - **NOTE**: This test encodes the expected behavior - it will validate the fix when it passes after implementation
  - **GOAL**: Surface counterexamples that demonstrate the bug exists
  - **Scoped PBT Approach**: For deterministic bugs, scope the property to concrete failing case(s) to ensure reproducibility
  - Test implementation details from Bug Condition in design:
    - Query: "create a travel vlog website using react" with 235 available skills
    - Query: "build a rest api with node.js and express" with 235 available skills
    - Query: "add authentication to my app" with empty project_type context
    - Query: "web development" with partial keyword matches
  - The test assertions should match the Expected Behavior Properties from design:
    - Assert selected_skills.count > 0 for common development queries
    - Assert selected_skills[0].confidence_score >= adjusted_threshold (0.3)
    - Assert selected_skills contains semantically relevant skills
  - Run test on UNFIXED code
  - **EXPECTED OUTCOME**: Test FAILS (this is correct - it proves the bug exists)
  - Document counterexamples found to understand root cause:
    - SkillSelector returns 0 skills for common development queries
    - Keyword scores are very low (0.0-0.2) due to exact matching requirements
    - Capability scores are 0 when query intent doesn't exactly match capability names
    - Context scores are 0 when project_type is empty or doesn't match exactly
  - Mark task complete when test is written, run, and failure is documented
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

## Phase 2: Preservation - Verify Non-Buggy Behavior

- [ ] 2. Write preservation property tests (BEFORE implementing fix)
  - **Property 2: Preservation** - Niche Queries and Edge Cases
  - **IMPORTANT**: Follow observation-first methodology
  - Observe behavior on UNFIXED code for non-buggy inputs (where isBugCondition returns false):
    - Niche Query: "implement a custom blockchain consensus algorithm" with 235 available skills (should return 0 skills)
    - Empty Query: "" with 235 available skills (should return 0 skills)
    - Disabled Skills: All skills disabled (should return 0 skills)
    - Very Specific Requirements: "implement OAuth 2.0 with PKCE flow" (should select only highly relevant skills if they exist)
  - Write property-based tests capturing observed behavior patterns from Preservation Requirements:
    - For queries with no relevant skills available, system SHALL CONTINUE TO return 0 selected skills
    - For queries with very specific/niche requirements, system SHALL CONTINUE TO select only highly relevant skills
    - For disabled skills, system SHALL CONTINUE TO exclude them from selection
    - For multiple sequential tasks, system SHALL CONTINUE TO maintain consistent scoring
  - Property-based testing generates many test cases for stronger guarantees
  - Run tests on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (this confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

## Phase 3: Implementation - Apply the Fix

- [ ] 3. Fix for skills selection algorithm
  - [ ] 3.1 Implement the fix
    - Modify `src-tauri/src/commands/skills/selector.rs`:
      - Update `calculate_keyword_score()` to use fuzzy matching instead of exact substring matching
        - Replace exact substring matching with Levenshtein distance or similar algorithm
        - Consider keywords as matching if they are similar enough (e.g., 80%+ similarity)
        - Allow "website" to match "web", "vlog" to match "video", etc.
        - Maintain backward compatibility: exact matches should still score highest
      - Update `calculate_capability_score()` to use semantic similarity and word-level matching
        - Replace exact substring matching with semantic similarity
        - Normalize capability names (convert "rest-api" to "rest api" for comparison)
        - Use word-level matching: if query intent contains any word that matches a capability word, count it as a match
        - Example: "rest api" query should match "rest-api" capability
      - Update `calculate_context_score()` to return neutral score when context is missing
        - When project_type is empty or unknown, return 0.5 instead of 0
        - This allows keyword and capability scores to determine skill selection when context is missing
        - When project_type is provided, use the existing matching logic
    - Modify `src-tauri/src/commands/skills/models.rs`:
      - Update `SkillsConfig::confidence_threshold` from 0.5 to 0.3 (30%)
    - _Bug_Condition: isBugCondition(input) where input.description CONTAINS common_development_terms AND available_skills_count > 0 AND selected_skills_count == 0_
    - _Expected_Behavior: For any task query where bug condition holds, SkillSelector SHALL select at least one relevant skill with confidence_score >= 0.3_
    - _Preservation: For any task query where bug condition does NOT hold, SkillSelector SHALL produce same result as original algorithm_
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4_

  - [ ] 3.2 Verify bug condition exploration test now passes
    - **Property 1: Expected Behavior** - Common Development Queries Select Skills
    - **IMPORTANT**: Re-run the SAME test from task 1 - do NOT write a new test
    - The test from task 1 encodes the expected behavior
    - When this test passes, it confirms the expected behavior is satisfied
    - Run bug condition exploration test from step 1
    - **EXPECTED OUTCOME**: Test PASSES (confirms bug is fixed)
    - Verify that:
      - Query "create a travel vlog website using react" now selects React, Web Development, UI/UX skills
      - Query "build a rest api with node.js and express" now selects Node.js, Express, REST API skills
      - Query "add authentication to my app" now selects authentication and security skills
      - Query "web development" now selects web development related skills
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [ ] 3.3 Verify preservation tests still pass
    - **Property 2: Preservation** - Niche Queries and Edge Cases
    - **IMPORTANT**: Re-run the SAME tests from task 2 - do NOT write new tests
    - Run preservation property tests from step 2
    - **EXPECTED OUTCOME**: Tests PASS (confirms no regressions)
    - Confirm all tests still pass after fix (no regressions):
      - Niche queries continue to return 0 skills when no relevant skills exist
      - Empty queries continue to return 0 skills
      - Disabled skills continue to be excluded
      - Very specific requirements continue to select only highly relevant skills
      - Scoring remains consistent across multiple sequential tasks
    - _Requirements: 3.1, 3.2, 3.3, 3.4_

## Phase 4: Checkpoint

- [ ] 4. Checkpoint - Ensure all tests pass
  - Verify all exploration tests pass (Property 1: Bug Condition)
  - Verify all preservation tests pass (Property 2: Preservation)
  - Verify no compilation errors in modified files
  - Verify no regressions in existing agent functionality
  - Ensure all tests pass, ask the user if questions arise
