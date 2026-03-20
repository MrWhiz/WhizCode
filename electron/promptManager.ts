// Dynamic Prompt Adaptation Manager for WhizCode
// Stores and selects prompt fragments based on project context

export interface PromptFragment {
  id: string;
  name: string;
  triggerPaths?: string[];      // Trigger if workspace has these folders/files
  triggerExtensions?: string[]; // Trigger if workspace has these extensions
  triggerKeywords?: string[];   // Trigger if user message has these keywords
  content: string;
}

export class PromptManager {
  private fragments: PromptFragment[] = [];

  constructor() {
    this.initializeFragments();
  }

  private initializeFragments() {
    this.fragments.push({
      id: 'react-standard',
      name: 'React & Next.js Best Practices',
      triggerExtensions: ['tsx', 'jsx'],
      triggerKeywords: ['react', 'nextjs', 'component', 'tailwind', 'hooks'],
      content: `
### React & Frontend Guidelines
- **Modern Paradigms**: Use Functional Components with Hooks. Prefer Server Components (Next.js) when appropriate.
- **Tailwind CSS**: If using Tailwind, use utility classes effectively. Avoid redundant nesting.
- **State Management**: Use React Context or simple hooks for local state. Prefer TanStack Query for data fetching.
- **Performance**: Use useMemo and useCallback only where necessary. Ensure keys are unique and stable.
- **Accessibility**: Use semantic HTML (h1, button, nav) and ARIA attributes where needed.
`
    });

    this.fragments.push({
      id: 'typescript-strict',
      name: 'TypeScript Type-Safety',
      triggerExtensions: ['ts', 'tsx'],
      content: `
### TypeScript & Type-Safety
- **No 'any'**: Avoid 'any' at all costs. Use 'unknown' or proper interfaces.
- **Interfaces vs Types**: Use 'interface' for objects that might be extended, and 'type' for unions/aliases.
- **Null Safety**: Always handle null/undefined explicitly. Use optional chaining (?.) and nullish coalescing (??).
- **Zod Validation**: If parsing external data, use Zod for schema validation.
`
    });

    this.fragments.push({
      id: 'node-electron',
      name: 'Node.js & Electron Safety',
      triggerPaths: ['electron', 'main.ts', 'preload.ts'],
      triggerKeywords: ['ipc', 'electron', 'renderer', 'main process'],
      content: `
### Node.js & Electron Guidelines
- **Process Separation**: Keep main process logic isolated from renderer logic.
- **IPC Security**: Only expose safe methods via ContextBridge. Avoid 'nodeIntegration: true'.
- **Async FS**: Use 'fs/promises' or 'fs.promises' for non-blocking file operations.
- **Error Handling**: Always use try-catch blocks for IPC handlers to prevent process crashes.
`
    });

    this.fragments.push({
      id: 'backend-python',
      name: 'Pythonic Excellence',
      triggerExtensions: ['py'],
      triggerKeywords: ['python', 'django', 'flask', 'fastapi'],
      content: `
### Python Guidelines (PEP-8)
- **Typing**: Use 'typing' modules for all function signatures.
- **Async**: Prefer 'asyncio' and 'httpx' for I/O bound tasks.
- **Environment**: Use 'venv' or 'poetry' for dependency management.
- **Docstrings**: Include Google or Sphinx style docstrings for complex functions.
`
    });

    this.fragments.push({
      id: 'mercurial-mermaid',
      name: 'Mermaid Documentation',
      triggerKeywords: ['diagram', 'architecture', 'flow', 'sequence', 'mermaid'],
      content: `
### Mermaid Diagramming
- **Visualization**: When explaining complex flows or architectures, use the 'generate_diagram' tool.
- **Syntax**: Ensure Mermaid syntax is valid for the diagram type (graph, sequenceDiagram, erDiagram).
`
    });
  }

  /**
   * Builds a custom prompt suffix based on the provided context
   */
  public getRelevantFragments(context: { 
    userMessage: string, 
    workspaceExtensions: string[], 
    workspacePaths: string[] 
  }): string {
    const selected = this.fragments.filter(f => {
      // Check extensions
      if (f.triggerExtensions?.some(ext => context.workspaceExtensions.includes(ext))) return true;
      
      // Check paths
      if (f.triggerPaths?.some(path => context.workspacePaths.some(wp => wp.includes(path)))) return true;
      
      // Check keywords
      const msg = context.userMessage.toLowerCase();
      if (f.triggerKeywords?.some(kw => msg.includes(kw))) return true;

      return false;
    });

    if (selected.length === 0) return '';

    let result = '\n\n## CONTEXT-SPECIFIC GUIDELINES (DYNAMIC)\n';
    result += selected.map(f => `#### ${f.name}\n${f.content.trim()}`).join('\n\n');
    return result;
  }
}
