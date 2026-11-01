import { defineConfig } from '@rsbuild/core';
import { pluginSass } from '@rsbuild/plugin-sass';
import { pluginSvelte } from '@rsbuild/plugin-svelte';

export default defineConfig({
  dev: {
    hmr: true,
    liveReload: true,
  },
  plugins: [
    pluginSvelte({
      svelteLoaderOptions: {
        emitCss: true,
        preprocess: [],
      },
    }),
    pluginSass(),
  ],
  html: {
    template: './public/index.html',
  },
  tools: {
    rspack: {
      module: {
        rules: [
          {
            test: /\.(glsl)$/,
            type: 'asset/source',
          },
        ],
      },
    },
  },
  splitChunks: {
    cacheGroups: {
      maplibre: {
        test: /maplibre-gl/,
        name: 'maplibre-shared',
        chunks: 'all',
      },
    },
  },
});
