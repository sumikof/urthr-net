import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // Some @solana/* (and transitive) modules read process.env.NODE_ENV at eval
  // time. The browser has no `process`, which throws "process is not defined".
  // Shim it so those references resolve harmlessly.
  define: {
    global: 'globalThis',
    'process.env': '{}',
  },
})
