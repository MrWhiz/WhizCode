---
description: Planning a fix and identifying dependencies
---

When receiving a user request, follow these steps to ensure a robust implementation plan:

1. **Information Gathering**:
   - Use `semantic_search` to find relevant logic in the codebase.
   - Read critical files to understand the current implementation.

2. **Analysis**:
   - Use `get_blast_radius` on any files you intend to modify to understand downstream impacts.
   - Check `package.json` and imports to determine if new external libraries (e.g., `axios`, `lodash`) are required.

3. **Manifest of Change**:
   - Before executing code changes, generate a internal plan (Thought) or a JSON-like structure that identifies:
     - **Files to create/edit**: A list of paths.
     - **Shell commands**: Any `npm install` or setup commands needed.

4. **Execution**:
   - Run required shell commands first.
   - Apply file changes using `write_file`, `insert_code`, or `replace_lines`.
