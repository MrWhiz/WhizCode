#!/usr/bin/env node

/**
 * Analyze node_modules size to identify optimization opportunities
 * Run: node analyze-bundle.cjs
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function getDirectorySize(dir) {
  let size = 0;
  try {
    const files = fs.readdirSync(dir, { withFileTypes: true });
    for (const file of files) {
      const fullPath = path.join(dir, file.name);
      if (file.isDirectory()) {
        size += getDirectorySize(fullPath);
      } else {
        size += fs.statSync(fullPath).size;
      }
    }
  } catch (e) {
    // Ignore errors
  }
  return size;
}

function formatBytes(bytes) {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}

const nodeModulesPath = path.join(__dirname, 'node_modules');

if (!fs.existsSync(nodeModulesPath)) {
  console.error('node_modules not found');
  process.exit(1);
}

console.log('Analyzing node_modules size...\n');

const packages = fs.readdirSync(nodeModulesPath, { withFileTypes: true })
  .filter(f => f.isDirectory() && !f.name.startsWith('.'))
  .map(f => ({
    name: f.name,
    size: getDirectorySize(path.join(nodeModulesPath, f.name))
  }))
  .sort((a, b) => b.size - a.size)
  .slice(0, 30);

console.log('Top 30 Largest Packages:\n');
console.log('Package Name'.padEnd(40) + 'Size'.padStart(15));
console.log('-'.repeat(55));

let totalSize = 0;
packages.forEach(pkg => {
  console.log(pkg.name.padEnd(40) + formatBytes(pkg.size).padStart(15));
  totalSize += pkg.size;
});

console.log('-'.repeat(55));
console.log('Total (top 30)'.padEnd(40) + formatBytes(totalSize).padStart(15));

// Identify heavy dependencies
const heavyDeps = packages.filter(p => p.size > 5 * 1024 * 1024);
console.log('\n\nHeavy Dependencies (>5MB):');
heavyDeps.forEach(dep => {
  console.log(`  - ${dep.name}: ${formatBytes(dep.size)}`);
});

console.log('\n\nOptimization Suggestions:');
console.log('1. Consider lazy-loading: @monaco-editor/react, mermaid, @xterm/xterm');
console.log('2. Check if all AWS SDK modules are needed');
console.log('3. Use tree-shaking for large libraries');
console.log('4. Consider lighter alternatives for heavy dependencies');
