import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import {defineConfig, loadEnv} from 'vite';

export default defineConfig(({mode}) => {
  const env = loadEnv(mode, '.', '');
  return {
    plugins: [react(), tailwindcss()],
    define: {
      'process.env.SDKWORK_ACCESS_TOKEN': JSON.stringify(env.SDKWORK_ACCESS_TOKEN ?? ''),
      'process.env.GEMINI_API_KEY': JSON.stringify(env.GEMINI_API_KEY),
    },
    resolve: {
      alias: [
        { find: '@sdkwork/modelkit-sdk-typescript', replacement: path.resolve(__dirname, './sdks/sdkwork-modelkit-sdk-typescript/src/index.ts') },
        { find: /^@sdkwork\/modelkit-(.*)$/, replacement: path.resolve(__dirname, './packages/sdkwork-modelkit-$1/src/index.ts') },
        { find: '@', replacement: path.resolve(__dirname, '.') }
      ]
    },
    server: {
      // HMR is disabled in AI Studio via DISABLE_HMR env var.
      // Do not modifyâfile watching is disabled to prevent flickering during agent edits.
      hmr: process.env.DISABLE_HMR !== 'true',
    },
  };
});
