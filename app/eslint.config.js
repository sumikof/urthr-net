import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  // Never lint generated code (codama output) or build artifacts.
  globalIgnores(['dist', 'src/generated']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      globals: globals.browser,
    },
    rules: {
      // This is a dev/debug harness; co-locating small helpers (parsers, the
      // shared client) with components is intentional. Fast-refresh DX is not a
      // correctness concern here, so surface it as a warning rather than an error.
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
    },
  },
])
