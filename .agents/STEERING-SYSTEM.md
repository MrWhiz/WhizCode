# Steering Files System

## Overview
Steering files provide custom instructions and guidelines that are automatically included in the agent's context, similar to Kiro's steering system.

## What are Steering Files?

Steering files are Markdown documents that contain:
- Coding standards and conventions
- Project-specific guidelines
- Best practices
- Architecture decisions
- Team norms

They help the agent understand your project's specific requirements without repeating instructions in every conversation.

## Inclusion Types

### 1. Always Included
Steering files that are included in every agent request.

**Use for:**
- General coding standards
- Company-wide conventions
- Security guidelines
- Common best practices

**Example:**
```markdown
---
inclusion: always
---

# Coding Standards

- Use TypeScript for all new code
- Follow ESLint rules strictly
- Write descriptive variable names
```

### 2. File Match (Conditional)
Steering files included only when working on files matching a pattern.

**Use for:**
- Framework-specific guidelines (React, Vue, etc.)
- File-type specific rules (API, components, tests)
- Directory-specific conventions

**Example:**
```markdown
---
inclusion: fileMatch
fileMatchPattern: ".*\\.(tsx|jsx)$"
---

# React Component Guidelines

- Use functional components with hooks
- Keep components small and focused
```

### 3. Manual
Steering files that must be explicitly referenced (future feature).

**Use for:**
- Rarely used guidelines
- Specialized instructions
- Optional conventions

**Example:**
```markdown
---
inclusion: manual
---

# API Conventions

Use this when working on API endpoints...
```

## File Structure

### Location
```
.kiro/steering/
  ├── coding-standards.md
  ├── react-guidelines.md
  ├── api-conventions.md
  └── testing-practices.md
```

### Format

```markdown
---
inclusion: always|fileMatch|manual
fileMatchPattern: "regex pattern"
enabled: true|false
---

# Your Instructions Here

Content in Markdown format...
```

### Front Matter Fields

- **inclusion**: When to include (always, fileMatch, manual)
- **fileMatchPattern**: Regex pattern for fileMatch (optional)
- **enabled**: Whether the steering is active (default: true)

## File References

You can reference other files in steering content:

```markdown
# API Guidelines

See the OpenAPI spec for details: #[[file:docs/api-spec.yaml]]

Follow the authentication flow in: #[[file:docs/auth-flow.md]]
```

The system will note these references in the context.

## Examples

### Example 1: General Coding Standards

**File:** `.kiro/steering/coding-standards.md`

```markdown
---
inclusion: always
---

# Coding Standards

## General Guidelines
- Use TypeScript for all new code
- Follow ESLint rules strictly
- Write descriptive variable names
- Add comments for complex logic

## Code Style
- Use 2 spaces for indentation
- Use single quotes for strings
- Add trailing commas in objects/arrays
- Use async/await instead of promises

## Testing
- Write tests for all new features
- Aim for 80%+ code coverage
- Use descriptive test names
```

### Example 2: React Guidelines

**File:** `.kiro/steering/react-guidelines.md`

```markdown
---
inclusion: fileMatch
fileMatchPattern: ".*\\.(tsx|jsx)$"
---

# React Component Guidelines

## Component Structure
- Use functional components with hooks
- Keep components small and focused
- Extract reusable logic into custom hooks

## Props
- Define prop types with TypeScript interfaces
- Use destructuring for props
- Provide default values when appropriate

## State Management
- Use useState for local state
- Use useContext for shared state
- Consider useReducer for complex state
```

### Example 3: Testing Practices

**File:** `.kiro/steering/testing-practices.md`

```markdown
---
inclusion: fileMatch
fileMatchPattern: ".*\\.test\\.(ts|tsx|js|jsx)$"
---

# Testing Practices

## Test Structure
- Use describe blocks for grouping
- Use it/test for individual tests
- Follow AAA pattern (Arrange, Act, Assert)

## Naming
- Test names should describe behavior
- Use "should" or "when" format
- Be specific about expected outcomes

## Coverage
- Test happy paths
- Test error cases
- Test edge cases
- Mock external dependencies
```

## How It Works

### Context Building

When the agent runs, steering context is automatically added:

```xml
<project_context>
  <workspace_root>/path/to/project</workspace_root>
  <file_tree>...</file_tree>
  <active_editor_file>...</active_editor_file>
</project_context>

<steering_instructions>
<!-- coding-standards.md -->
# Coding Standards
...
</steering_instructions>

<file_specific_instructions>
<!-- react-guidelines.md (matched: src/App.tsx) -->
# React Component Guidelines
...
</file_specific_instructions>
```

### Matching Logic

1. **Always included**: Added to every request
2. **File match**: Regex tested against active file path
3. **Manual**: Not yet implemented (future feature)

### Regex Patterns

Common patterns for fileMatchPattern:

```
React files:        ".*\\.(tsx|jsx)$"
TypeScript files:   ".*\\.tsx?$"
Test files:         ".*\\.test\\.(ts|tsx|js|jsx)$"
API files:          ".*/api/.*\\.ts$"
Component files:    ".*/components/.*\\.(tsx|jsx)$"
Specific directory: "^src/features/.*"
```

## Usage

### Via IPC (Programmatic)

```javascript
// List all steering files
const files = await ipcRenderer.invoke('steering:list');

// Get specific steering file
const steering = await ipcRenderer.invoke('steering:get', 'coding-standards');

// Save steering file
await ipcRenderer.invoke('steering:save', {
  id: 'my-guidelines',
  name: 'my-guidelines.md',
  content: '# My Guidelines\n...',
  inclusion: 'always',
  enabled: true
});

// Delete steering file
await ipcRenderer.invoke('steering:delete', 'my-guidelines');

// Reload from disk
await ipcRenderer.invoke('steering:reload');
```

### Via File System

1. Create a Markdown file in `.kiro/steering/`
2. Add front matter with inclusion type
3. Write your guidelines in Markdown
4. Reload steering or restart app

## Best Practices

### Content
- Be specific and actionable
- Use examples
- Keep it concise
- Update regularly

### Organization
- One topic per file
- Use descriptive filenames
- Group related guidelines
- Version control your steering files

### Performance
- Don't include too many always-included files
- Use fileMatch for specific guidelines
- Keep content focused and relevant

## Benefits

### Consistency
- Agent follows your standards automatically
- No need to repeat instructions
- Team alignment on conventions

### Efficiency
- Faster onboarding for new team members
- Reduced back-and-forth with agent
- Consistent code quality

### Flexibility
- Different rules for different file types
- Easy to update and maintain
- Version controlled with your code

## Comparison with Kiro

| Feature | WhizCode | Kiro |
|---------|----------|------|
| Always Inclusion | ✅ | ✅ |
| File Match | ✅ | ✅ |
| Manual Inclusion | ⏳ Planned | ✅ |
| File References | ✅ Basic | ✅ Full |
| Front Matter | ✅ | ✅ |
| Markdown Format | ✅ | ✅ |
| UI Management | ⏳ Planned | ✅ |

## Implementation Status

✅ Core steering system
✅ Front matter parsing
✅ Always inclusion
✅ File match inclusion
✅ File reference notation
✅ IPC handlers
✅ Context integration
⏳ Manual inclusion (TODO)
⏳ Full file reference resolution (TODO)
⏳ UI for management (TODO)

## Future Enhancements

### Planned
- Manual inclusion with # syntax
- Full file reference resolution
- UI for managing steering files
- Steering templates
- Steering validation

### Possible
- Conditional logic in steering
- Steering inheritance
- Steering priorities
- Steering analytics
- Team sharing
