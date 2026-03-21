# WhizCode Build Size Optimization Summary

## Current Optimizations Applied

### 1. Package.json Build Configuration
- **Aggressive file exclusions**: Removed all `.json`, `.ts`, `.map`, `.d.ts`, `.wasm`, `.node` files from node_modules
- **Differential packaging**: Enabled for smaller incremental updates
- **Maximum compression**: NSIS uses maximum compression algorithm
- **Smart unpacking**: Only unpacks necessary files at runtime

### 2. Vite Configuration
- **Code splitting**: Monaco, Mermaid, xterm, and markdown libraries split into separate chunks
- **Terser minification**: Aggressive minification with console/debugger removal
- **Manual chunks**: Large dependencies isolated for better compression

### 3. Node Modules Cleanup
Excluded from build:
- Documentation files (*.md, *.markdown)
- Source files (*.ts, *.c, *.cc, *.cpp, *.h)
- Type definitions (*.d.ts)
- Source maps (*.map)
- Config files (tsconfig.json, jest.config, webpack.config, etc.)
- Lock files (package-lock.json, yarn.lock, pnpm-lock.yaml)
- Binary files (*.wasm, *.node) - except node-pty
- Build directories (dist/, build/)
- GitHub files (.github/, .gitignore, .eslintrc)

## Further Optimization Strategies

### Strategy 1: Remove Unused Dependencies (Recommended)
Analyze which dependencies are actually used:

```bash
# Install npm-check-updates
npm install -g npm-check-updates

# Check for unused packages
npm prune --production
```

**Candidates for removal:**
- `@aws-sdk/client-bedrock-runtime` (~5-10MB) - Only if not using AWS Bedrock
- `mermaid` (~3-5MB) - Only if diagram rendering not needed
- `voyageai` (~2MB) - Only if not using Voyage AI embeddings
- `react-syntax-highlighter` (~2MB) - If using Monaco instead

### Strategy 2: Lazy Load Heavy Dependencies
Load large libraries only when needed:

```typescript
// Instead of importing at top
import mermaid from 'mermaid';

// Lazy load when needed
const mermaid = await import('mermaid');
```

### Strategy 3: Use Lighter Alternatives
- Replace `mermaid` with `graphviz-wasm` (~1MB instead of 5MB)
- Replace `react-syntax-highlighter` with `highlight.js` (~500KB instead of 2MB)
- Use `@monaco-editor/loader` for dynamic loading

### Strategy 4: Optimize Native Modules
```json
{
  "extraResources": [
    {
      "from": "node_modules/node-pty/build/Release",
      "to": "build/Release",
      "filter": ["*.node"]  // Only include .node files
    }
  ]
}
```

### Strategy 5: Enable Delta Updates
For future releases, use differential packages:
```json
{
  "nsis": {
    "differentialPackage": true
  }
}
```

## Build Size Breakdown

Typical size distribution (99MB installer):
- Electron runtime: ~40-50MB
- Node modules: ~30-40MB
- Application code: ~5-10MB
- Assets: ~5-10MB

## Recommended Actions

### Immediate (5-10MB savings)
1. Remove unused AWS SDK if not needed
2. Remove voyageai if not using embeddings
3. Clean up node_modules with `npm prune`

### Short-term (10-20MB savings)
1. Lazy load Mermaid and Monaco
2. Replace heavy dependencies with lighter alternatives
3. Enable code splitting for all large libraries

### Long-term (20-30MB savings)
1. Consider using a lighter editor (CodeMirror instead of Monaco)
2. Implement plugin system for optional features
3. Use WebAssembly for performance-critical code

## Build Commands

```bash
# Clean build
npm run prebuild

# Build with optimizations
npm run build

# Package with NSIS
npm run package

# Check bundle size
npm run build -- --analyze  # If using rollup-plugin-visualizer
```

## Monitoring Build Size

Add this to package.json scripts:
```json
{
  "analyze": "vite build --analyze"
}
```

Then install:
```bash
npm install --save-dev rollup-plugin-visualizer
```

Update vite.config.ts:
```typescript
import { visualizer } from 'rollup-plugin-visualizer';

export default defineConfig({
  plugins: [
    visualizer({
      open: true,
      gzipSize: true,
      brotliSize: true,
    })
  ]
})
```

## Estimated Size Reductions

| Strategy | Savings | Effort |
|----------|---------|--------|
| Remove unused deps | 10-20MB | Low |
| Lazy load heavy libs | 5-10MB | Medium |
| Lighter alternatives | 10-15MB | Medium |
| Code splitting | 2-5MB | Low |
| Optimize node_modules | 5-10MB | Low |
| **Total Potential** | **32-60MB** | - |

## Testing After Optimization

1. Verify all features work
2. Check startup time
3. Monitor memory usage
4. Test file operations
5. Verify terminal functionality

## Performance vs Size Trade-off

- **Aggressive minification**: Slightly slower startup, smaller size
- **Code splitting**: Slower initial load, faster feature access
- **Lazy loading**: Slower feature first-use, smaller initial download

Current config balances all three for optimal user experience.

## Next Steps

1. Run `npm run package` with current optimizations
2. Check installer size
3. If still >80MB, implement Strategy 1 (remove unused deps)
4. If still >60MB, implement Strategy 2 (lazy loading)
5. If still >50MB, implement Strategy 3 (lighter alternatives)
