use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub workspace_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    pub cycle: Vec<String>,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub symbol_id: String,
    pub direct_dependents: Vec<String>,
    pub transitive_dependents: Vec<String>,
    pub blast_radius: usize,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReachabilityAnalysis {
    pub from_symbol: String,
    pub reachable_symbols: Vec<String>,
    pub reachability_score: f32,
}

#[allow(dead_code)]
pub struct GraphService {
    graphs: Arc<Mutex<HashMap<String, DependencyGraph>>>,
}

#[allow(dead_code)]
impl GraphService {
    pub fn new() -> Self {
        Self {
            graphs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn build_dependency_graph(&self, workspace_path: String, nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> Result<DependencyGraph> {
        let graph = DependencyGraph {
            nodes,
            edges,
            workspace_path: workspace_path.clone(),
        };

        let mut graphs = self.graphs.lock().unwrap();
        graphs.insert(workspace_path, graph.clone());
        Ok(graph)
    }

    pub fn find_circular_dependencies(&self, workspace_path: &str) -> Vec<CircularDependency> {
        let graphs = self.graphs.lock().unwrap();
        let mut cycles = vec![];

        if let Some(graph) = graphs.get(workspace_path) {
            let mut visited = HashSet::new();
            let mut rec_stack = HashSet::new();

            for node in &graph.nodes {
                if !visited.contains(&node.id) {
                    let mut path = vec![];
                    if self.has_cycle_dfs(&graph.edges, &node.id, &mut visited, &mut rec_stack, &mut path) {
                        cycles.push(CircularDependency {
                            cycle: path,
                            severity: "high".to_string(),
                        });
                    }
                }
            }
        }

        cycles
    }

    fn has_cycle_dfs(
        &self,
        edges: &[GraphEdge],
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        for edge in edges {
            if edge.from == node {
                if !visited.contains(&edge.to) {
                    if self.has_cycle_dfs(edges, &edge.to, visited, rec_stack, path) {
                        return true;
                    }
                } else if rec_stack.contains(&edge.to) {
                    path.push(edge.to.clone());
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        false
    }

    pub fn analyze_impact(&self, workspace_path: &str, symbol_id: &str) -> ImpactAnalysis {
        let graphs = self.graphs.lock().unwrap();
        let mut direct_dependents = vec![];
        let mut transitive_dependents = HashSet::new();

        if let Some(graph) = graphs.get(workspace_path) {
            for edge in &graph.edges {
                if edge.to == symbol_id {
                    direct_dependents.push(edge.from.clone());
                }
            }

            let mut queue = VecDeque::new();
            for dependent in &direct_dependents {
                queue.push_back(dependent.clone());
            }

            while let Some(current) = queue.pop_front() {
                for edge in &graph.edges {
                    if edge.to == current && !transitive_dependents.contains(&edge.from) {
                        transitive_dependents.insert(edge.from.clone());
                        queue.push_back(edge.from.clone());
                    }
                }
            }
        }

        let blast_radius = direct_dependents.len() + transitive_dependents.len();
        let risk_level = match blast_radius {
            0..=5 => "low".to_string(),
            6..=15 => "medium".to_string(),
            _ => "high".to_string(),
        };

        ImpactAnalysis {
            symbol_id: symbol_id.to_string(),
            direct_dependents,
            transitive_dependents: transitive_dependents.into_iter().collect(),
            blast_radius,
            risk_level,
        }
    }

    pub fn analyze_reachability(&self, workspace_path: &str, from_symbol: &str) -> ReachabilityAnalysis {
        let graphs = self.graphs.lock().unwrap();
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(from_symbol.to_string());
        reachable.insert(from_symbol.to_string());

        if let Some(graph) = graphs.get(workspace_path) {
            while let Some(current) = queue.pop_front() {
                for edge in &graph.edges {
                    if edge.from == current && !reachable.contains(&edge.to) {
                        reachable.insert(edge.to.clone());
                        queue.push_back(edge.to.clone());
                    }
                }
            }
        }

        let total_nodes = graphs
            .get(workspace_path)
            .map(|g| g.nodes.len())
            .unwrap_or(1);

        let reachability_score = (reachable.len() as f32) / (total_nodes as f32).max(1.0);

        ReachabilityAnalysis {
            from_symbol: from_symbol.to_string(),
            reachable_symbols: reachable.into_iter().collect(),
            reachability_score,
        }
    }

    pub fn get_graph(&self, workspace_path: &str) -> Option<DependencyGraph> {
        let graphs = self.graphs.lock().unwrap();
        graphs.get(workspace_path).cloned()
    }

    pub fn clear_graph(&self, workspace_path: &str) {
        let mut graphs = self.graphs.lock().unwrap();
        graphs.remove(workspace_path);
    }
}

#[tauri::command]
pub async fn graph_build_dependency_graph(
    workspace_path: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
) -> Result<DependencyGraph> {
    eprintln!("Building dependency graph for: {}", workspace_path);
    Ok(DependencyGraph {
        nodes,
        edges,
        workspace_path,
    })
}

#[tauri::command]
pub async fn graph_find_circular_dependencies(workspace_path: String) -> Result<Vec<CircularDependency>> {
    eprintln!("Finding circular dependencies in: {}", workspace_path);
    Ok(vec![])
}

#[tauri::command]
pub async fn graph_analyze_impact(workspace_path: String, symbol_id: String) -> Result<ImpactAnalysis> {
    eprintln!("Analyzing impact of symbol: {} in {}", symbol_id, workspace_path);
    Ok(ImpactAnalysis {
        symbol_id,
        direct_dependents: vec![],
        transitive_dependents: vec![],
        blast_radius: 0,
        risk_level: "low".to_string(),
    })
}

#[tauri::command]
pub async fn graph_analyze_reachability(workspace_path: String, from_symbol: String) -> Result<ReachabilityAnalysis> {
    eprintln!("Analyzing reachability from: {} in {}", from_symbol, workspace_path);
    Ok(ReachabilityAnalysis {
        from_symbol,
        reachable_symbols: vec![],
        reachability_score: 0.0,
    })
}

#[tauri::command]
pub async fn graph_get_graph(workspace_path: String) -> Result<Option<DependencyGraph>> {
    eprintln!("Getting dependency graph for: {}", workspace_path);
    Ok(None)
}

#[tauri::command]
pub async fn graph_clear_graph(workspace_path: String) -> Result<()> {
    eprintln!("Clearing dependency graph for: {}", workspace_path);
    Ok(())
}
