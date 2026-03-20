import { defineConfig } from 'vite'
import path from 'node:path'
import electron from 'vite-plugin-electron/simple'
import react from '@vitejs/plugin-react'
import { builtinModules } from 'node:module'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    electron({
      main: {
        entry: 'electron/main.ts',
        vite: {
          build: {
            minify: 'terser',
            rollupOptions: {
              external: [
                'electron',
                ...builtinModules,
                ...builtinModules.map(m => `node:${m}`),
                'node-pty',
                'tree-sitter',
                'tree-sitter-typescript'
              ],
            },
          },
        },
      },
      preload: {
        input: path.join(__dirname, 'electron/preload.ts'),
        vite: {
          build: {
            minify: 'terser',
            rollupOptions: {
              external: [
                'electron',
                ...builtinModules,
                ...builtinModules.map(m => `node:${m}`),
                'node-pty',
                'tree-sitter',
                'tree-sitter-typescript'
              ],
            },
          },
        },
      },
      renderer: {},
    }),
  ],
  build: {
    minify: 'terser',
    rollupOptions: {
      output: {
        manualChunks: {
          'monaco': ['@monaco-editor/react'],
          'mermaid': ['mermaid'],
          'xterm': ['@xterm/xterm', '@xterm/addon-fit'],
          'markdown': ['react-markdown', 'remark-gfm'],
        },
        chunkFileNames: 'chunks/[name]-[hash].js',
        entryFileNames: '[name]-[hash].js',
      },
    },
    chunkSizeWarningLimit: 1000,
    reportCompressedSize: false,
  },
})
