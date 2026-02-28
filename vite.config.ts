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
            rollupOptions: {
              external: [
                'electron',
                ...builtinModules,
                ...builtinModules.map(m => `node:${m}`),
                'node-pty',
                '@lancedb/lancedb',
                'tree-sitter',
                'tree-sitter-typescript',
                'lancedb'
              ],
            },
          },
        },
      },
      preload: {
        input: path.join(__dirname, 'electron/preload.ts'),
        vite: {
          build: {
            rollupOptions: {
              external: [
                'electron',
                ...builtinModules,
                ...builtinModules.map(m => `node:${m}`),
                'node-pty',
                '@lancedb/lancedb',
                'tree-sitter',
                'tree-sitter-typescript',
                'lancedb'
              ],
            },
          },
        },
      },
      renderer: {},
    }),
  ],
})
