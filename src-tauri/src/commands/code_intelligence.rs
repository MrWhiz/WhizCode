use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;
use crate::error::Result;
use crate::utils;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub id: String,
    pub name: String,
    pub symbol_type: String,
    pub file_path: String,
    pub line_number: u32,
    pub scope: String,
    pub dependencies: Vec<String>,
    pub complexity: f32,
    pub last_modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRelationship {
    pub from_symbol: String,
    pub to_symbol: String,
    pub relationship_type: String,
    pub strength: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    pub id: String,
    pub pattern_name: String,
    pub description: String,
    pub occurrences: u32,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    pub total_files: u32,
    pub total_symbols: u32,
    pub average_complexity: f32,
    pub cohesion_score: f32,
    pub technical_debt: f32,
    pub maintainability_index: f32,
    pub cyclomatic_complexity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringRecommendation {
    pub file_path: String,
    pub recommendation: String,
    pub priority: String,
    pub estimated_effort: f32,
    pub impact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContext {
    pub workspace_path: String,
    pub symbols: Vec<CodeSymbol>,
    pub relationships: Vec<CodeRelationship>,
    pub patterns: Vec<CodePattern>,
    pub metrics: CodeMetrics,
    pub last_analyzed: u64,
    pub scan_signature: u64,
}

pub struct CodeIntelligence {
    contexts: Arc<Mutex<HashMap<String, SemanticContext>>>,
}

impl CodeIntelligence {
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn workspace_scan_signature(workspace_path: &str) -> u64 {
        let mut latest_seen = 0u64;
        let mut file_count = 0u64;

        for entry in WalkDir::new(workspace_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            if utils::should_skip_file(path) {
                continue;
            }

            file_count += 1;
            let modified = fs::metadata(path)
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            latest_seen = latest_seen.max(modified);
        }

        latest_seen ^ file_count
    }

    pub fn analyze_workspace_if_stale(&self, workspace_path: String) -> Result<SemanticContext> {
        let current_signature = Self::workspace_scan_signature(&workspace_path);
        if let Some(existing) = self.contexts.lock().unwrap().get(&workspace_path).cloned() {
            if existing.scan_signature == current_signature {
                return Ok(existing);
            }
        }

        self.analyze_workspace(workspace_path)
    }

    pub fn analyze_workspace(&self, workspace_path: String) -> Result<SemanticContext> {
        eprintln!("[INTEL] Analyzing workspace: {}", workspace_path);

        let mut symbols = Vec::new();
        let mut file_count = 0;
        let mut total_complexity = 0.0;
        let scan_signature = Self::workspace_scan_signature(&workspace_path);

        for entry in WalkDir::new(&workspace_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            if utils::should_skip_file(path) {
                continue;
            }

            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(path) {
                let file_path = path.to_string_lossy().to_string();
                let file_symbols = self.extract_symbols(&file_path, &content);
                for mut symbol in file_symbols {
                    symbol.complexity = self.estimate_complexity(&content);
                    total_complexity += symbol.complexity;
                    symbols.push(symbol);
                }
                file_count += 1;
            }
        }

        let total_symbols = symbols.len() as u32;
        let avg_complexity = if total_symbols > 0 { total_complexity / total_symbols as f32 } else { 0.0 };

        let mut context = SemanticContext {
            workspace_path: workspace_path.clone(),
            symbols: symbols.clone(),
            relationships: vec![],
            patterns: self.extract_patterns(&symbols),
            metrics: CodeMetrics {
                total_files: file_count,
                total_symbols,
                average_complexity: avg_complexity,
                cohesion_score: 0.85,
                technical_debt: 15.0,
                maintainability_index: 85.0,
                cyclomatic_complexity: avg_complexity * 1.5,
            },
            last_analyzed: Self::current_timestamp(),
            scan_signature,
        };

        // ── 2. RELATIONSHIP ANALYSIS ─────────────────────────────────────
        context.relationships = self.analyze_relationships(&context);

        let mut contexts = self.contexts.lock().unwrap();
        contexts.insert(workspace_path, context.clone());
        
        eprintln!("[INTEL] Finished analysis: {} files, {} symbols", file_count, total_symbols);
        Ok(context)
    }

    pub fn update_file(&self, workspace_path: &str, file_path: &str) -> Result<()> {
        let full_path = std::path::Path::new(file_path);
        let resolved_path = if full_path.is_absolute() {
            full_path.to_path_buf()
        } else {
            std::path::Path::new(workspace_path).join(full_path)
        };

        if !resolved_path.exists() {
            return self.remove_file(workspace_path, file_path);
        }

        let content = match fs::read_to_string(&resolved_path) {
            Ok(content) => content,
            Err(_) => {
                return self.remove_file(workspace_path, file_path);
            }
        };

        let file_path = resolved_path.to_string_lossy().replace('\\', "/");

        let mut contexts = self.contexts.lock().unwrap();
        let context = if let Some(existing) = contexts.get_mut(workspace_path) {
            existing
        } else {
            drop(contexts);
            return self.analyze_workspace(workspace_path.to_string()).map(|_| ());
        };

        let old_symbols: Vec<String> = context
            .symbols
            .iter()
            .filter(|symbol| symbol.file_path == file_path)
            .map(|symbol| symbol.name.clone())
            .collect();

        context.symbols.retain(|symbol| symbol.file_path != file_path);

        let mut refreshed_symbols = self.extract_symbols(&file_path, &content);
        let complexity = self.estimate_complexity(&content);
        for symbol in &mut refreshed_symbols {
            symbol.complexity = complexity;
        }
        context.symbols.extend(refreshed_symbols.clone());
        context.patterns = self.extract_patterns(&context.symbols);

        context.relationships.retain(|relationship| {
            let references_removed_file = relationship.from_symbol == file_path
                || relationship.to_symbol.starts_with(&format!("{}:", file_path));
            let references_removed_symbol = old_symbols.iter().any(|symbol_name| {
                relationship.to_symbol.ends_with(&format!(":{}", symbol_name))
                    || relationship.from_symbol.contains(symbol_name)
            });
            !(references_removed_file || references_removed_symbol)
        });

        let current_symbols = context.symbols.clone();
        for symbol in &current_symbols {
            if symbol.file_path == file_path {
                continue;
            }
            if content.contains(&symbol.name) {
                context.relationships.push(CodeRelationship {
                    from_symbol: file_path.clone(),
                    to_symbol: format!("{}:{}", symbol.file_path, symbol.name),
                    relationship_type: "references".to_string(),
                    strength: 1.0,
                });
            }
        }

        context.metrics = Self::calculate_metrics(&context.symbols);
        context.last_analyzed = Self::current_timestamp();
        context.scan_signature = Self::workspace_scan_signature(workspace_path);

        eprintln!("[INTEL] Incrementally refreshed symbols for {}", file_path);
        Ok(())
    }

    pub fn remove_file(&self, workspace_path: &str, file_path: &str) -> Result<()> {
        let path = std::path::Path::new(file_path);
        let resolved_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::path::Path::new(workspace_path).join(path)
        };
        let file_key = resolved_path.to_string_lossy().replace('\\', "/");

        let mut contexts = self.contexts.lock().unwrap();
        let Some(context) = contexts.get_mut(workspace_path) else {
            return Ok(());
        };

        let removed_symbols: Vec<String> = context
            .symbols
            .iter()
            .filter(|symbol| symbol.file_path == file_key)
            .map(|symbol| symbol.name.clone())
            .collect();

        context.symbols.retain(|symbol| symbol.file_path != file_key);
        context.relationships.retain(|relationship| {
            let references_removed_file = relationship.from_symbol == file_key
                || relationship.to_symbol.starts_with(&format!("{}:", file_key));
            let references_removed_symbol = removed_symbols.iter().any(|symbol_name| {
                relationship.to_symbol.ends_with(&format!(":{}", symbol_name))
                    || relationship.from_symbol.contains(symbol_name)
            });
            !(references_removed_file || references_removed_symbol)
        });
        context.patterns = self.extract_patterns(&context.symbols);
        context.metrics = Self::calculate_metrics(&context.symbols);
        context.last_analyzed = Self::current_timestamp();
        context.scan_signature = Self::workspace_scan_signature(workspace_path);

        eprintln!("[INTEL] Removed cached symbols for {}", file_key);
        Ok(())
    }

    fn analyze_relationships(&self, context: &SemanticContext) -> Vec<CodeRelationship> {
        let mut relationships = Vec::new();
        let mut symbol_to_file: HashMap<String, String> = HashMap::new();
        
        // 1. Map symbols to their original defining files
        for symbol in &context.symbols {
            symbol_to_file.insert(symbol.name.clone(), symbol.file_path.clone());
        }

        // 2. Scan every file for usage of other files' symbols
        let ws_root = &context.workspace_path;
        for entry in WalkDir::new(ws_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            if crate::utils::should_skip_file(path) { continue; }
            
            if let Ok(content) = std::fs::read_to_string(path) {
                let current_file = path.to_string_lossy().to_string();
                
                for (sym_name, sym_file) in &symbol_to_file {
                    if *sym_file == current_file { continue; } // Skip self-references
                    
                    // Fast check for symbol name in content
                    if content.contains(sym_name) {
                        relationships.push(CodeRelationship {
                            from_symbol: current_file.clone(), // In this graph, 'from' is the user
                            to_symbol: format!("{}:{}", sym_file, sym_name), // 'to' is the definition
                            relationship_type: "references".to_string(),
                            strength: 1.0,
                        });
                    }
                }
            }
        }

        relationships
    }

    fn calculate_metrics(symbols: &[CodeSymbol]) -> CodeMetrics {
        let total_files = symbols
            .iter()
            .map(|symbol| symbol.file_path.clone())
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;
        let total_symbols = symbols.len() as u32;
        let average_complexity = if total_symbols > 0 {
            symbols.iter().map(|symbol| symbol.complexity).sum::<f32>() / total_symbols as f32
        } else {
            0.0
        };

        CodeMetrics {
            total_files,
            total_symbols,
            average_complexity,
            cohesion_score: 0.85,
            technical_debt: 15.0,
            maintainability_index: 85.0,
            cyclomatic_complexity: average_complexity * 1.5,
        }
    }

    fn extract_patterns(&self, symbols: &[CodeSymbol]) -> Vec<CodePattern> {
        let mut patterns = Vec::new();
        let large_files = symbols.iter().fold(HashMap::new(), |mut acc, symbol| {
            *acc.entry(symbol.file_path.clone()).or_insert(0u32) += 1;
            acc
        });

        for (file_path, count) in large_files {
            if count >= 10 {
                patterns.push(CodePattern {
                    id: format!("dense-symbols:{}", file_path),
                    pattern_name: "dense-symbol-cluster".to_string(),
                    description: format!("{} declares {} symbols and may benefit from decomposition", file_path, count),
                    occurrences: count,
                    severity: "medium".to_string(),
                });
            }
        }

        patterns
    }

    fn extract_symbols(&self, file_path: &str, content: &str) -> Vec<CodeSymbol> {
        let mut symbols = Vec::new();
        
        // Simple regex-like extraction (using contains/starts_with for speed in this implementation)
        for (i, line) in content.lines().enumerate() {
            let line_num = (i + 1) as u32;
            let trimmed = line.trim();
            
            let (symbol_name, symbol_type) = if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || trimmed.starts_with("async fn ") {
                 (self.parse_name(trimmed, "fn "), "function")
            } else if trimmed.starts_with("class ") || trimmed.starts_with("export class ") {
                 (self.parse_name(trimmed, "class "), "class")
            } else if trimmed.starts_with("interface ") || trimmed.starts_with("export interface ") {
                 (self.parse_name(trimmed, "interface "), "interface")
            } else if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
                 (self.parse_name(trimmed, "struct "), "struct")
            } else if trimmed.starts_with("def ") {
                 (self.parse_name(trimmed, "def "), "function")
            } else {
                continue;
            };

            if let Some(name) = symbol_name {
                symbols.push(CodeSymbol {
                    id: format!("{}_{}_{}", file_path, name, line_num),
                    name: name.to_string(),
                    symbol_type: symbol_type.to_string(),
                    file_path: file_path.to_string(),
                    line_number: line_num,
                    scope: "global".to_string(),
                    dependencies: vec![],
                    complexity: self.estimate_complexity(line),
                    last_modified: Self::current_timestamp(),
                });
            }
        }
        
        symbols
    }

    fn parse_name(&self, line: &str, keyword: &str) -> Option<String> {
        let after_keyword = line.split(keyword).nth(1)?;
        let name = after_keyword.split(|c| c == '(' || c == '{' || c == '<' || c == ' ' || c == ':').next()?;
        if name.trim().is_empty() { None } else { Some(name.trim().to_string()) }
    }

    fn estimate_complexity(&self, line: &str) -> f32 {
        let mut c = 1.0;
        if line.contains("if") { c += 1.0; }
        if line.contains("for") { c += 1.0; }
        if line.contains("while") { c += 1.0; }
        if line.contains("match") { c += 2.0; }
        if line.contains("?") { c += 0.5; }
        c
    }

    pub fn get_symbol_info(&self, workspace_path: &str, symbol_name: &str) -> Option<CodeSymbol> {
        self.contexts.lock().unwrap()
            .get(workspace_path)
            .and_then(|ctx| {
                ctx.symbols
                    .iter()
                    .find(|s| s.name == symbol_name)
                    .cloned()
            })
    }

    pub fn get_all_symbols(&self, workspace_path: &str) -> Vec<CodeSymbol> {
        let contexts = self.contexts.lock().unwrap();
        contexts
            .get(workspace_path)
            .map(|ctx| ctx.symbols.clone())
            .unwrap_or_default()
    }

    pub fn get_code_metrics(&self, workspace_path: &str) -> Option<CodeMetrics> {
        let contexts = self.contexts.lock().unwrap();
        contexts
            .get(workspace_path)
            .map(|ctx| ctx.metrics.clone())
    }

    pub fn suggest_refactoring(&self, workspace_path: &str, file_path: &str) -> Vec<RefactoringRecommendation> {
        let mut suggestions = vec![];
        let contexts = self.contexts.lock().unwrap();

        if let Some(ctx) = contexts.get(workspace_path) {
            if ctx.metrics.average_complexity > 10.0 {
                suggestions.push(RefactoringRecommendation {
                    file_path: file_path.to_string(),
                    recommendation: "Consider breaking down complex functions".to_string(),
                    priority: "high".to_string(),
                    estimated_effort: 4.0,
                    impact: "Improves maintainability and testability".to_string(),
                });
            }
        }
        suggestions
    }

    pub fn get_all_relationships(&self, workspace_path: &str) -> Vec<CodeRelationship> {
        self.contexts
            .lock()
            .unwrap()
            .get(workspace_path)
            .map(|ctx| ctx.relationships.clone())
            .unwrap_or_default()
    }

    pub fn get_all_patterns(&self, workspace_path: &str) -> Vec<CodePattern> {
        self.contexts
            .lock()
            .unwrap()
            .get(workspace_path)
            .map(|ctx| ctx.patterns.clone())
            .unwrap_or_default()
    }

    pub fn find_related_symbols(&self, workspace_path: &str, symbol_name: &str) -> Vec<CodeSymbol> {
        let contexts = self.contexts.lock().unwrap();
        let Some(ctx) = contexts.get(workspace_path) else {
            return Vec::new();
        };

        let mut related_names = Vec::new();
        for relationship in &ctx.relationships {
            if relationship.to_symbol.ends_with(&format!(":{}", symbol_name)) {
                if let Some(source_file) = relationship.from_symbol.split(':').next() {
                    related_names.push(source_file.to_string());
                }
            }
            if relationship.from_symbol.ends_with(symbol_name) {
                related_names.push(relationship.to_symbol.clone());
            }
        }

        ctx.symbols
            .iter()
            .filter(|symbol| {
                symbol.name == symbol_name
                    || related_names.iter().any(|candidate| {
                        candidate.contains(&symbol.name) || candidate == &symbol.file_path
                    })
            })
            .cloned()
            .collect()
    }

    pub fn impact_analysis(&self, workspace_path: &str, symbol_id: &str) -> HashMap<String, Vec<String>> {
        let contexts = self.contexts.lock().unwrap();
        let Some(ctx) = contexts.get(workspace_path) else {
            return HashMap::new();
        };

        let mut impact = HashMap::new();
        if let Some(symbol) = ctx.symbols.iter().find(|item| item.id == symbol_id || item.name == symbol_id) {
            impact.insert("symbol".to_string(), vec![symbol.name.clone()]);
            impact.insert(
                "files".to_string(),
                ctx.relationships
                    .iter()
                    .filter(|rel| rel.to_symbol.contains(&symbol.name))
                    .map(|rel| rel.from_symbol.clone())
                    .collect(),
            );
        }

        impact
    }
}

// Tauri commands
#[tauri::command]
pub async fn code_intelligence_analyze_workspace(
    workspace_path: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<CodeIntelligence>>>,
) -> Result<SemanticContext> {
    let intel = state.lock().unwrap();
    intel.analyze_workspace_if_stale(workspace_path)
}

#[tauri::command]
pub async fn code_intelligence_get_symbol_info(
    workspace_path: String,
    symbol_name: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<CodeIntelligence>>>,
) -> Result<Option<CodeSymbol>> {
    let intel = state.lock().unwrap();
    Ok(intel.get_symbol_info(&workspace_path, &symbol_name))
}

#[tauri::command]
pub async fn code_intelligence_get_all_symbols(
    workspace_path: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<CodeIntelligence>>>,
) -> Result<Vec<CodeSymbol>> {
    let intel = state.lock().unwrap();
    Ok(intel.get_all_symbols(&workspace_path))
}

#[tauri::command]
pub async fn code_intelligence_get_metrics(
    workspace_path: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<CodeIntelligence>>>,
) -> Result<Option<CodeMetrics>> {
    let intel = state.lock().unwrap();
    Ok(intel.get_code_metrics(&workspace_path))
}

#[tauri::command]
pub async fn code_intelligence_suggest_refactoring(
    workspace_path: String,
    file_path: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<CodeIntelligence>>>,
) -> Result<Vec<RefactoringRecommendation>> {
    let intel = state.lock().unwrap();
    Ok(intel.suggest_refactoring(&workspace_path, &file_path))
}

#[tauri::command]
pub async fn code_intelligence_find_related_symbols(
    workspace_path: String,
    symbol_name: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<CodeIntelligence>>>,
) -> Result<Vec<CodeSymbol>> {
    let intel = state.lock().unwrap();
    Ok(intel.find_related_symbols(&workspace_path, &symbol_name))
}

#[tauri::command]
pub async fn code_intelligence_get_all_relationships(
    workspace_path: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<CodeIntelligence>>>,
) -> Result<Vec<CodeRelationship>> {
    let intel = state.lock().unwrap();
    Ok(intel.get_all_relationships(&workspace_path))
}

#[tauri::command]
pub async fn code_intelligence_get_all_patterns(
    workspace_path: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<CodeIntelligence>>>,
) -> Result<Vec<CodePattern>> {
    let intel = state.lock().unwrap();
    Ok(intel.get_all_patterns(&workspace_path))
}

#[tauri::command]
pub async fn code_intelligence_find_circular_dependencies(_workspace_path: String) -> Result<Vec<Vec<String>>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn code_intelligence_impact_analysis(
    workspace_path: String,
    symbol_id: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<CodeIntelligence>>>,
) -> Result<HashMap<String, Vec<String>>> {
    let intel = state.lock().unwrap();
    Ok(intel.impact_analysis(&workspace_path, &symbol_id))
}
