import {defineConfig} from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri 移动端：单页应用，资源经 asset protocol 提供，必须用相对 base
export default defineConfig({
    plugins: [vue()],
    base: './',
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
    },
    build: {
        outDir: 'dist',
        emptyOutDir: true,
        target: 'es2021',
    },
})
