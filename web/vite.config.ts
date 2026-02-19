import { defineConfig, loadEnv } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

function normalizeBasePath(rawBasePath: string | undefined): string {
  const trimmed = rawBasePath?.trim();
  if (!trimmed || trimmed === './') {
    return '/';
  }

  const withLeadingSlash = trimmed.startsWith('/') ? trimmed : `/${trimmed}`;
  return withLeadingSlash.endsWith('/') ? withLeadingSlash : `${withLeadingSlash}/`;
}

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, '.', '');

  return {
    plugins: [svelte()],
    base: normalizeBasePath(env.VITE_BASE_PATH),
    define: {
      global: 'globalThis'
    },
    optimizeDeps: {
      esbuildOptions: {
        define: {
          global: 'globalThis'
        }
      }
    },
    server: {
      host: '0.0.0.0',
      port: 5173
    }
  };
});
