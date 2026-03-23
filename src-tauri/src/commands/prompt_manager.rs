use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromptFragment {
    pub id: String,
    pub name: String,
    pub trigger_paths: Option<Vec<String>>,
    pub trigger_extensions: Option<Vec<String>>,
    pub trigger_keywords: Option<Vec<String>>,
    pub content: String,
}

pub struct PromptManager {
    fragments: Vec<PromptFragment>,
}

impl PromptManager {
    pub fn new() -> Self {
        let mut fragments = Vec::new();

        fragments.push(PromptFragment {
            id: "react-standard".to_string(),
            name: "React & Next.js Best Practices".to_string(),
            trigger_paths: None,
            trigger_extensions: Some(vec!["tsx".to_string(), "jsx".to_string()]),
            trigger_keywords: Some(vec!["react".to_string(), "nextjs".to_string(), "component".to_string(), "tailwind".to_string(), "hooks".to_string()]),
            content: "
### React & Frontend Guidelines
- **Modern Paradigms**: Use Functional Components with Hooks. Prefer Server Components (Next.js) when appropriate.
- **Tailwind CSS**: If using Tailwind, use utility classes effectively. Avoid redundant nesting.
- **State Management**: Use React Context or simple hooks for local state. Prefer TanStack Query for data fetching.
- **Performance**: Use useMemo and useCallback only where necessary. Ensure keys are unique and stable.
- **Accessibility**: Use semantic HTML (h1, button, nav) and ARIA attributes where needed.
".trim().to_string(),
        });

        fragments.push(PromptFragment {
            id: "typescript-strict".to_string(),
            name: "TypeScript Type-Safety".to_string(),
            trigger_extensions: Some(vec!["ts".to_string(), "tsx".to_string()]),
            content: "
### TypeScript & Type-Safety
- **No 'any'**: Avoid 'any' at all costs. Use 'unknown' or proper interfaces.
- **Interfaces vs Types**: Use 'interface' for objects that might be extended, and 'type' for unions/aliases.
- **Null Safety**: Always handle null/undefined explicitly. Use optional chaining (?.) and nullish coalescing (??).
- **Zod Validation**: If parsing external data, use Zod for schema validation.
".trim().to_string(),
            trigger_paths: None,
            trigger_keywords: None,
        });

        fragments.push(PromptFragment {
            id: "node-tauri".to_string(),
            name: "Node.js & Tauri Safety".to_string(),
            trigger_paths: Some(vec!["src-tauri".to_string(), "main.rs".to_string()]),
            trigger_keywords: Some(vec!["tauri".to_string(), "rust".to_string(), "command".to_string(), "backend".to_string()]),
            content: "
### Node.js & Tauri Guidelines
- **Process Separation**: Keep Tauri commands isolated from frontend logic.
- **IPC Security**: Only expose safe methods via Tauri's invoke system.
- **Error Handling**: Always return Result types in Rust commands to prevent process panics.
- **Async Safety**: Use tokio::sync or equivalent for cross-thread communication in Rust.
".trim().to_string(),
            trigger_extensions: None,
        });

        fragments.push(PromptFragment {
            id: "python-excellence".to_string(),
            name: "Pythonic Excellence".to_string(),
            trigger_paths: None,
            trigger_extensions: Some(vec!["py".to_string()]),
            trigger_keywords: Some(vec!["python".to_string(), "django".to_string(), "flask".to_string(), "fastapi".to_string()]),
            content: "
### Python Guidelines (PEP-8)
- **Typing**: Use 'typing' modules for all function signatures.
- **Async**: Prefer 'asyncio' and 'httpx' for I/O bound tasks.
- **Environment**: Use 'venv' or 'poetry' for dependency management.
- **Docstrings**: Include Google or Sphinx style docstrings for complex functions.
".trim().to_string(),
        });

        fragments.push(PromptFragment {
            id: "mermaid-docs".to_string(),
            name: "Mermaid Documentation".to_string(),
            trigger_keywords: Some(vec!["diagram".to_string(), "architecture".to_string(), "flow".to_string(), "sequence".to_string(), "mermaid".to_string()]),
            content: "
### Mermaid Diagramming
- **Visualization**: When explaining complex flows or architectures, use the 'generate_diagram' tool.
- **Syntax**: Ensure Mermaid syntax is valid for the diagram type (graph, sequenceDiagram, erDiagram).
".trim().to_string(),
            trigger_paths: None,
            trigger_extensions: None,
        });

        PromptManager { fragments }
    }

    pub fn get_relevant_fragments(&self, user_message: &str, extensions: &[String], paths: &[String]) -> String {
        let mut selected = Vec::new();
        let msg_lower = user_message.to_lowercase();
        let ext_set: HashSet<_> = extensions.iter().collect();

        for f in &self.fragments {
            let mut triggered = false;

            // Check extensions
            if let Some(trigger_exts) = &f.trigger_extensions {
                if trigger_exts.iter().any(|ext| ext_set.contains(ext)) {
                    triggered = true;
                }
            }

            // Check paths
            if !triggered {
                if let Some(trigger_paths) = &f.trigger_paths {
                    if trigger_paths.iter().any(|p| paths.iter().any(|path| path.contains(p))) {
                        triggered = true;
                    }
                }
            }

            // Check keywords
            if !triggered {
                if let Some(trigger_kws) = &f.trigger_keywords {
                    if trigger_kws.iter().any(|kw| msg_lower.contains(kw)) {
                        triggered = true;
                    }
                }
            }

            if triggered {
                selected.push(f);
            }
        }

        if selected.is_empty() {
            return String::new();
        }

        let mut result = String::from("\n\n## CONTEXT-SPECIFIC GUIDELINES (DYNAMIC)\n");
        for f in selected {
            result.push_str(&format!("#### {}\n{}\n\n", f.name, f.content));
        }
        result
    }
}
