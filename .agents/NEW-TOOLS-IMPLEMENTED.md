# New Tools Implemented

## Overview
Added 9 new powerful tools to WhizCode, bringing the total from 15 to 24 tools. These tools bring the IDE much closer to Kiro's capabilities.

## New Tools

### 1. readCode
**Purpose:** Read files with AST-based structure analysis  
**Parameters:** `path`  
**Returns:** File content + extracted structure (classes, functions, arrow functions with line numbers)  
**Use Case:** When you need to understand the structure of a file before editing

```json
{
  "tool": "readCode",
  "path": "src/components/App.tsx"
}
```

### 2. editCode
**Purpose:** AST-aware code editing  
**Parameters:** `path`, `search`, `replace`  
**Returns:** Success message with data about the edit  
**Use Case:** When you need to make precise code edits without worrying about exact whitespace

```json
{
  "tool": "editCode",
  "path": "src/components/App.tsx",
  "search": "const App = () =>",
  "replace": "const App = () => {"
}
```

### 3. getDiagnostics
**Purpose:** Get TypeScript/ESLint errors  
**Parameters:** `path`  
**Returns:** Diagnostics output or success message  
**Use Case:** Check for errors before/after making changes

```json
{
  "tool": "getDiagnostics",
  "path": "src/utils/helpers.ts"
}
```

### 4. grepSearch
**Purpose:** Fast regex search with line numbers  
**Parameters:** `pattern`, `include` (optional), `maxResults` (default: 50)  
**Returns:** Match results with file paths and line numbers  
**Use Case:** When you need to find patterns across files quickly

```json
{
  "tool": "grepSearch",
  "pattern": "TODO|FIXME",
  "include": "*.ts"
}
```

### 5. fileSearch
**Purpose:** Fuzzy file finding  
**Parameters:** `query`, `maxResults` (default: 10)  
**Returns:** Matching files with relevance scores  
**Use Case:** When you know part of a filename but not the full path

```json
{
  "tool": "fileSearch",
  "query": "user-card"
}
```

### 6. readMultipleFiles
**Purpose:** Read many files at once  
**Parameters:** `files` (array of paths)  
**Returns:** Content of all files with line numbers  
**Use Case:** When you need to understand multiple files at once

```json
{
  "tool": "readMultipleFiles",
  "files": ["src/components/App.tsx", "src/components/Button.tsx", "src/utils/helpers.ts"]
}
```

### 7. semanticRename
**Purpose:** Rename symbols with automatic reference updates  
**Parameters:** `path`, `oldName`, `newName`  
**Returns:** Summary of renamed locations  
**Use Case:** When renaming variables, functions, or classes

```json
{
  "tool": "semanticRename",
  "path": "src/components/UserCard.tsx",
  "oldName": "user",
  "newName": "userData"
}
```

### 8. smartRelocate
**Purpose:** Move files with automatic import updates  
**Parameters:** `sourcePath`, `destinationPath`  
**Returns:** Summary of moved file and updated imports  
**Use Case:** When refactoring file structure

```json
{
  "tool": "smartRelocate",
  "sourcePath": "src/components/UserCard.tsx",
  "destinationPath": "src/components/User/UserCard.tsx"
}
```

### 9. strReplace
**Purpose:** Precise string replacement  
**Parameters:** `path`, `oldStr`, `newStr`  
**Returns:** Success message with data about the replacement  
**Use Case:** When you need to replace exact strings (not regex)

```json
{
  "tool": "strReplace",
  "path": "src/components/App.tsx",
  "oldStr": "import React from 'react'",
  "newStr": "import React, { useState } from 'react'"
}
```

## Tool Selection Guide

### For Reading Files:
- **read_file** - Simple file reading with line numbers
- **readCode** - File reading with AST structure analysis
- **readMultipleFiles** - Read many files at once

### For Editing Files:
- **edit_file** - String-based edits (exact match required)
- **editCode** - AST-aware edits (better for code)
- **strReplace** - Simple string replacement
- **write_file** - Complete file rewrite

### For Searching:
- **search_files** - Basic pattern search
- **grepSearch** - Fast regex search with line numbers
- **fileSearch** - Fuzzy file finding
- **semantic_search** - Vector-based code search

### For Analysis:
- **getDiagnostics** - TypeScript/ESLint errors
- **get_blast_radius** - Dependency analysis
- **readCode** - Structure analysis

### For Refactoring:
- **semanticRename** - Rename symbols with reference updates
- **smartRelocate** - Move files with import updates

## Examples

### Example 1: Add a new component
```json
{
  "tool": "fileSearch",
  "query": "components"
}
```
```json
{
  "tool": "readCode",
  "path": "src/components/App.tsx"
}
```
```json
{
  "tool": "editCode",
  "path": "src/components/App.tsx",
  "search": "const App = () =>",
  "replace": "const App = () => {\n  return (\n    <div>\n      <h1>Welcome</h1>\n      <NewComponent />\n    </div>\n  );"
}
```

### Example 2: Find and fix TODOs
```json
{
  "tool": "grepSearch",
  "pattern": "TODO|FIXME",
  "include": "*.ts"
}
```

### Example 3: Rename a variable across the project
```json
{
  "tool": "semanticRename",
  "path": "src/utils/helpers.ts",
  "oldName": "data",
  "newName": "userData"
}
```

### Example 4: Move a component to a subfolder
```json
{
  "tool": "smartRelocate",
  "sourcePath": "src/components/UserCard.tsx",
  "destinationPath": "src/components/User/UserCard.tsx"
}
```

## Performance Notes

- **grepSearch** is faster than search_files for large codebases
- **fileSearch** uses fuzzy matching with scoring
- **readMultipleFiles** reads files in parallel
- **semanticRename** and **smartRelocate** scan the entire workspace

## Error Handling

All new tools include proper error handling:
- File not found errors
- Binary file detection
- Invalid parameters
- Permission errors

## Integration with Agent

The agent now has access to 24 tools and will automatically choose the most appropriate one based on the task. The system prompt has been updated to include guidance on when to use each tool.
