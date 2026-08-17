import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: { proxy: { "/ws": { target: "ws://localhost:3000", ws: true }, "/qr.svg": "http://localhost:3000" } },
  build: { rollupOptions: { output: { manualChunks: (id) => id.includes("/phaser/") ? "phaser" : undefined } } },
});
