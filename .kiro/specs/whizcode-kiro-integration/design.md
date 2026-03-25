# WhizCode Behavior Enhancement - Design Document (Optimized)

## Overview

Enhance WhizCode to behave exactly like Kiro by leveraging existing infrastructure and reusing proven functions. This design minimizes duplication and maximizes code reuse.

## Architecture

### System Components

```
Frontend (React)
    ↓
User Query
    ↓
Tauri Command: execute_agent_loop_streaming
    ↓
WhizCode Integration Layer (Rust)
    ├─ Query Analyzer (reuse: whizcode_integration.rs)
    ├─ Workflow Router (reuse: whizcode_integration.rs)
    ├─ Prompt Optimizer (reuse: prompt_manager.rs + whizcode_integration.rs)
    ├─ Context Optimizer (reuse: context_optimizer.rs + vector_search.rs)
    └─ Streaming Feedback (reuse: streaming_feedback.rs)
    ↓
SubAgent Orchestration (reuse: sub_agents.rs)
    ├─ Query Analyzer SubAgent (delegate to existing)
    ├─ Workflow Router SubAgent (delegate to existing)
    ├─ Prompt Optimizer SubAgent (delegate to existing)
    ├─ Context Optimizer SubAgent (delegate to existing)
    ├─ Bugfix Workflow SubAgent (delegate to existing)
    ├─ Feature Implementation SubAgent (delegate to existing)
    ├─ Spec Creation SubAgent (delegate to existing)
    ├─ Refactoring SubAgent (delegate to existing)
    └─ Analysis SubAgent (reuse: code_intelligence.rs)
    ↓
Local LLM (Ollama)
    ↓
Streaming Response
    ↓
Tool Execution
    ↓
Frontend Updates
```

### Core Modules (Reusing Existing Code)

#### 1. WhizCode Integration Layer (`src-tauri/src/commands/whizcode_integration.rs`)

**Already Implemented - Reuse Directly**:
- `analyze_query()` - Query classification and requirement extraction
- `generate_optimized_prompt()` - Prompt generation
- `prune_context()` - Context reduction
- `route_query()` - Workflow routing

**Optimization**: These functions are already optimized and tested. No changes needed.

#### 2. Streaming Feedback (`src-tauri/src/commands/streaming_feedback.rs`)

**Already Implemented - Reuse Directly**:
- `start_streaming()` - Begin streaming session
- `add_token()` - Stream tokens
- `transition_phase()` - Change phase
- `get_metrics()` - Get metrics
- `end_streaming()` - End session

**Optimization**: These functions provide real-time feedback. No changes needed.

#### 3. Context Optimizer (`src-tauri/src/commands/context_optimizer.rs`)

**Already Implemented - Reuse Directly**:
- `prune_context()` - Optimize context
- `score_relevance()` - Score file relevance
- `create_file_summary()` - Summarize files
- `estimate_tokens()` - Estimate tokens

**Optimization**: These functions intelligently reduce context. No changes needed.

#### 4. Prompt Manager (`src-tauri/src/commands/prompt_manager.rs`)

**Existing Functions to Reuse**:
- `get_relevant_fragments()` - Get context-specific prompt fragments
- `PromptFragment` - Reusable prompt components

**Optimization**: Extend to support WhizCode-specific fragments for different query types.

#### 5. Code Intelligence (`src-tauri/src/commands/code_intelligence.rs`)

**Existing Functions to Reuse**:
- `analyze_workspace()` - Analyze codebase structure
- `extract_symbols()` - Extract code symbols
- `get_code_metrics()` - Get code metrics
- `suggest_refactoring()` - Suggest refactoring opportunities

**Optimization**: Use for Analysis SubAgent and Refactoring SubAgent.

#### 6. Vector Search (`src-tauri/src/commands/vector_search.rs`)

**Existing Functions to Reuse**:
- `semantic_search()` - Find relevant code sections
- `build_file_tree()` - Build file tree for context
- `get_index_stats()` - Get index statistics

**Optimization**: Use for context optimization and relevance scoring.

#### 7. SubAgent System (`src-tauri/src/commands/sub_agents.rs`)

**Existing Functions to Reuse**:
- `SubAgentExecutor` - Execute SubAgents
- `execute_sub_agent()` - Run SubAgent with task
- `SubAgentConfig` - SubAgent configuration

**Optimization**: Reuse existing SubAgent infrastructure. No new SubAgent system needed.

#### 8. Tool Result Cache (`src-tauri/src/commands/tool_result_cache.rs`)

**Existing Functions to Reuse**:
- `get()` - Get cached result
- `set()` - Cache result
- `invalidate()` - Invalidate cache

**Optimization**: Use for caching prompt optimizations and context pruning results.

### SubAgent Specifications (Reusing Existing Infrastructure)

All SubAgents will be registered in the existing `prompts.rs` module using `SubAgentConfig`. No new SubAgent system needed.

#### Query Analyzer SubAgent
- **Reuse**: `whizcode_integration.rs::analyze_query()`
- **System Prompt**: Define in prompts.rs
- **Execution**: Via existing SubAgentExecutor

#### Workflow Router SubAgent
- **Reuse**: `whizcode_integration.rs::route_query()`
- **System Prompt**: Define in prompts.rs
- **Execution**: Via existing SubAgentExecutor

#### Prompt Optimizer SubAgent
- **Reuse**: `whizcode_integration.rs::generate_optimized_prompt()` + `prompt_manager.rs::get_relevant_fragments()`
- **System Prompt**: Define in prompts.rs
- **Execution**: Via existing SubAgentExecutor

#### Context Optimizer SubAgent
- **Reuse**: `context_optimizer.rs::prune_context()` + `vector_search.rs::semantic_search()`
- **System Prompt**: Define in prompts.rs
- **Execution**: Via existing SubAgentExecutor

#### Bugfix Workflow SubAgent
- **Reuse**: Existing SubAgent infrastructure
- **System Prompt**: Define in prompts.rs
- **Execution**: Via existing SubAgentExecutor

#### Feature Implementation SubAgent
- **Reuse**: Existing SubAgent infrastructure
- **System Prompt**: Define in prompts.rs
- **Execution**: Via existing SubAgentExecutor

#### Spec Creation SubAgent
- **Reuse**: Existing SubAgent infrastructure
- **System Prompt**: Define in prompts.rs
- **Execution**: Via existing SubAgentExecutor

#### Refactoring SubAgent
- **Reuse**: `code_intelligence.rs::suggest_refactoring()` + `code_intelligence.rs::get_code_metrics()`
- **System Prompt**: Define in prompts.rs
- **Execution**: Via existing SubAgentExecutor

#### Analysis SubAgent
- **Reuse**: `code_intelligence.rs::analyze_workspace()` + `code_intelligence.rs::get_all_symbols()`
- **System Prompt**: Define in prompts.rs
- **Execution**: Via existing SubAgentExecutor

#### 1. WhizCode Integration Layer (`src-tauri/src/commands/whizcode_integration.rs`)

**Responsibility**: Orchestrate all WhizCode behavior operations

**Key Structures**:
- `QueryAnalysis` - Analyzes user queries for type, confidence, requirements, complexity
- `OptimizedPrompt` - Generates token-efficient system and user prompts
- `PrunedContext` - Reduces context size intelligently
- `WorkflowRoute` - Routes queries to appropriate workflows
- `WhizCodeIntegrationLayer` - Main orchestrator with static methods

**Key Methods**:
- `analyze_query(query: &str) -> QueryAnalysis` - Classify query and extract requirements
- `generate_optimized_prompt(query, query_type, context_size) -> OptimizedPrompt` - Create efficient prompts
- `prune_context(files, query, max_tokens) -> PrunedContext` - Reduce context intelligently
- `route_query(query, query_type) -> WorkflowRoute` - Route to appropriate workflow

#### 2. Streaming Feedback (`src-tauri/src/commands/streaming_feedback.rs`)

**Responsibility**: Provide real-time feedback during LLM processing

**Key Features**:
- Token streaming with immediate display
- Phase transition indicators (analyzing, planning, researching, executing, validating, thinking, processing, generating, loading)
- Metrics calculation (tokens/sec, estimated time remaining)
- Progress tracking

**Key Methods**:
- `new() -> Self` - Initialize streaming feedback
- `start_streaming()` - Begin streaming session
- `add_token(token: &str)` - Add token to stream
- `transition_phase(phase: &str)` - Change current phase
- `get_metrics() -> StreamingMetrics` - Get current metrics
- `end_streaming() -> StreamingMetrics` - End session and return final metrics

#### 3. Context Optimizer (`src-tauri/src/commands/context_optimizer.rs`)

**Responsibility**: Intelligently reduce context size for local LLM

**Key Features**:
- File relevance scoring based on query
- File summarization (first 50 lines + key sections)
- Code snippet extraction
- Token estimation and caching

**Key Methods**:
- `new(max_tokens: Option<u32>) -> Self` - Initialize with optional token limit
- `prune_context(files, query, workspace_path) -> PrunedContext` - Prune files intelligently
- `score_relevance(path, content, query) -> f32` - Score file relevance (0.0-1.0)
- `create_file_summary(path, content) -> String` - Summarize file content
- `estimate_tokens(content) -> u32` - Estimate token count

### SubAgent Specifications

#### Query Analyzer SubAgent
**Purpose**: Analyze and classify user queries
**Inputs**: User query string
**Outputs**: Query type, confidence, requirements, complexity
**Workflow**: 
1. Extract keywords and intent
2. Classify query type
3. Extract requirements
4. Assess complexity
5. Calculate confidence score

#### Workflow Router SubAgent
**Purpose**: Route queries to appropriate workflows
**Inputs**: Query type, analysis results
**Outputs**: Workflow name, agent name, prerequisites
**Workflow**:
1. Determine workflow based on query type
2. Validate prerequisites
3. Prepare workflow context
4. Return routing information

#### Prompt Optimizer SubAgent
**Purpose**: Generate token-efficient prompts
**Inputs**: Query, query type, context size
**Outputs**: Optimized system and user prompts
**Workflow**:
1. Build system prompt based on query type
2. Build user prompt with context
3. Estimate token count
4. Optimize if needed
5. Return optimized prompts

#### Context Optimizer SubAgent
**Purpose**: Optimize context for local LLM
**Inputs**: Files, query, max tokens
**Outputs**: Pruned context with relevant files
**Workflow**:
1. Score file relevance
2. Sort by relevance
3. Select files within token limit
4. Summarize large files
5. Return optimized context

#### Bugfix Workflow SubAgent
**Purpose**: Fix bugs using bug condition methodology
**Inputs**: Bug description, codebase context
**Outputs**: Bug fix implementation
**Workflow**:
1. Identify bug condition
2. Create exploration tests
3. Locate root cause
4. Implement fix
5. Verify fix works

#### Feature Implementation SubAgent
**Purpose**: Implement new features
**Inputs**: Feature requirements, codebase context
**Outputs**: Feature implementation
**Workflow**:
1. Analyze requirements
2. Design architecture
3. Implement code
4. Create tests
5. Validate against requirements

#### Spec Creation SubAgent
**Purpose**: Create specifications and requirements
**Inputs**: Feature description, project context
**Outputs**: Specification document
**Workflow**:
1. Gather requirements
2. Create design document
3. Break down into tasks
4. Define correctness properties
5. Return specification

#### Refactoring SubAgent
**Purpose**: Refactor code for improvement
**Inputs**: Code to refactor, improvement goals
**Outputs**: Refactored code
**Workflow**:
1. Analyze code structure
2. Identify improvements
3. Implement changes
4. Ensure functionality preserved
5. Return refactored code

#### Analysis SubAgent
**Purpose**: Analyze codebase and provide insights
**Inputs**: Codebase context, analysis query
**Outputs**: Analysis results and insights
**Workflow**:
1. Analyze codebase structure
2. Identify patterns
3. Generate insights
4. Suggest improvements
5. Return analysis results

### Integration Points

#### 1. Agent Streaming Command

**File**: `src-tauri/src/commands/agent_streaming.rs`

**Current**: `execute_agent_loop_streaming` delegates to `StreamingAgentOrchestrator`

**Integration Points**:
1. Before LLM call: Use `WhizCodeIntegrationLayer::analyze_query()` to classify query
2. Before LLM call: Use `ContextOptimizer` to reduce context size
3. Before LLM call: Use `WhizCodeIntegrationLayer::generate_optimized_prompt()` to create efficient prompts
4. During LLM call: Use `StreamingFeedback` to provide real-time feedback
5. After analysis: Use `WhizCodeIntegrationLayer::route_query()` to determine workflow

#### 2. Frontend Components

**Files**: 
- `src/components/Chat/ChatPanel.tsx` - Display streaming feedback
- `src/components/Chat/StreamingStatus.tsx` - Show phase transitions and metrics
- `src/hooks/useAppEventListeners.ts` - Listen for WhizCode events

**Updates**:
- Listen for `whizcode:phase_change` events
- Listen for `whizcode:metrics` events
- Display current phase and progress
- Show estimated time remaining
- Display tokens/sec metrics

#### 3. Tauri Command Wrappers

**File**: `src-tauri/src/commands/whizcode_integration.rs`

**New Commands**:
- `#[tauri::command] analyze_query(query: String) -> QueryAnalysis`
- `#[tauri::command] generate_optimized_prompt(query, query_type, context_size) -> OptimizedPrompt`
- `#[tauri::command] prune_context(files, query, workspace_path, max_tokens) -> PrunedContext`
- `#[tauri::command] route_query(query, query_type) -> WorkflowRoute`

#### 4. Main.rs Registration

**File**: `src-tauri/src/main.rs`

**Updates**:
- Register new Tauri commands in `invoke_handler`
- Ensure WhizCode modules are properly initialized

## Data Flow

### Query Processing Flow

```
1. User submits query
   ↓
2. Frontend calls execute_agent_loop_streaming
   ↓
3. Backend analyzes query with WhizCodeIntegrationLayer::analyze_query()
   - Determines query type (bugfix, feature, refactor, analysis, spec)
   - Extracts requirements
   - Assesses complexity
   - Calculates confidence score
   ↓
4. Backend optimizes context with ContextOptimizer
   - Scores file relevance
   - Selects most relevant files
   - Summarizes file content
   - Estimates total tokens
   ↓
5. Backend generates optimized prompt with WhizCodeIntegrationLayer::generate_optimized_prompt()
   - Creates system prompt based on query type
   - Creates user prompt with optimized context
   - Estimates token count
   ↓
6. Backend routes query with WhizCodeIntegrationLayer::route_query()
   - Determines appropriate workflow
   - Prepares workflow context
   ↓
7. Backend initializes StreamingFeedback
   - Starts streaming session
   - Begins phase tracking
   ↓
8. Backend calls local LLM with optimized prompt
   - Streams tokens in real-time
   - Updates phase as needed
   - Emits metrics to frontend
   ↓
9. Frontend receives streaming updates
   - Displays current phase
   - Shows metrics (tokens/sec, ETA)
   - Updates UI incrementally
   ↓
10. Backend processes LLM response
    - Executes tools as needed
    - Maintains conversation history
    ↓
11. Frontend displays final response
```

## Correctness Properties

### Property 1: Query Analysis Accuracy
- For any query, `analyze_query()` must correctly identify the query type
- Confidence score must be >= 0.7 for correctly classified queries
- Extracted requirements must be relevant to the query

### Property 2: Context Optimization Effectiveness
- Optimized context must reduce token count by at least 30% compared to full context
- Optimized context must include all files with relevance score > 0.3
- Optimized context must not exceed max_tokens limit

### Property 3: Prompt Optimization Efficiency
- Optimized prompt must use fewer tokens than naive prompt
- Optimized prompt must preserve all essential information
- Optimized prompt must be valid for the target LLM

### Property 4: Streaming Feedback Responsiveness
- Streaming metrics must update at least once per second
- Phase transitions must be emitted within 100ms
- Tokens must be streamed within 50ms of arrival

### Property 5: Workflow Routing Correctness
- Query type must match suggested workflow
- Workflow prerequisites must be validated
- Workflow context must be complete and accurate

## Implementation Constraints

### Performance Requirements
- Query analysis: < 100ms
- Context optimization: < 500ms
- Prompt optimization: < 100ms
- Workflow routing: < 50ms
- Total pre-LLM processing: < 1 second

### Token Limits
- Default context limit: 8192 tokens
- System prompt: < 500 tokens
- User prompt: < 2000 tokens
- Total prompt: < 2500 tokens

### Backward Compatibility
- All existing features must remain functional
- No breaking changes to existing APIs
- Graceful fallback if WhizCode components fail
- Existing conversation history must be preserved

## Integration Sequence

### Phase 1: Core Integration
1. Add Tauri command wrappers for WhizCode operations
2. Register commands in main.rs
3. Update execute_agent_loop_streaming to use WhizCode components
4. Test query analysis and routing

### Phase 2: Context Optimization
1. Integrate ContextOptimizer into agent_streaming
2. Verify context reduction effectiveness
3. Test with various file sizes and query types
4. Optimize relevance scoring

### Phase 3: Prompt Optimization
1. Integrate prompt optimizer into agent_streaming
2. Verify prompt efficiency
3. Test with local LLM
4. Measure token reduction

### Phase 4: Streaming Feedback
1. Integrate StreamingFeedback into agent_streaming
2. Emit phase change events to frontend
3. Emit metrics events to frontend
4. Update frontend components to display feedback

### Phase 5: Frontend Updates
1. Update ChatPanel to listen for WhizCode events
2. Update StreamingStatus to display phases and metrics
3. Add visual indicators for streaming progress
4. Test end-to-end flow

### Phase 6: Testing & Validation
1. Test with various query types
2. Verify correctness properties
3. Performance testing
4. User acceptance testing

## Success Criteria

- Query analysis works for all query types
- Context optimization reduces tokens by 30%+
- Prompt optimization reduces tokens by 20%+
- Streaming feedback updates in real-time
- Local LLM response time < 5 seconds for simple queries
- All existing features remain functional
- No breaking changes to APIs
- User perceives no "hanging" with streaming feedback

## Risk Mitigation

### Risk: WhizCode components fail during execution
**Mitigation**: Implement graceful fallback to existing behavior

### Risk: Context optimization removes important files
**Mitigation**: Implement relevance scoring validation and manual override

### Risk: Prompt optimization breaks LLM understanding
**Mitigation**: Test with multiple LLM models and validate output quality

### Risk: Streaming feedback causes performance issues
**Mitigation**: Implement efficient event batching and throttling

### Risk: Integration breaks existing features
**Mitigation**: Comprehensive testing before deployment, feature flags for gradual rollout
