import { defineConfig } from '@rspack/cli';
import { rspack } from '@rspack/core';
import { ReactRefreshRspackPlugin } from '@rspack/plugin-react-refresh';
import path from 'path';

const reactDevToolsPlugin = () => {
  return {
    name: 'react-devtools-injector',
    apply: (compiler: any) => {
      if (compiler.options.mode === 'development') {
        compiler.hooks.compilation.tap('ReactDevTools', (compilation: any) => {
          const hooks = rspack.HtmlRspackPlugin.getCompilationHooks(compilation);
          hooks.alterAssetTags.tapPromise('ReactDevTools', async (data) => {
            data.assetTags.scripts.unshift({
              tagName: 'script',
              attributes: {
                src: 'http://localhost:8097',
                defer: false,
                async: false,
              },
              voidTag: false,
            });
            console.log('React DevTools script tag added');
            return data;
          });
        });
      }
    },
  };
};

const isDev = process.env.NODE_ENV === 'development';

// Target browsers, see: https://github.com/browserslist/browserslist
const targets = ['last 2 versions', '> 0.2%', 'not dead', 'Firefox ESR'];

export default defineConfig({
  entry: {
    main: './src/main.tsx',
  },
  resolve: {
    extensions: ['...', '.ts', '.tsx', '.jsx'],
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  module: {
    rules: [
      {
        test: /\.css$/,
        use: ['postcss-loader'],
        type: 'css',
      },
      {
        test: /\.(png|jpe?g|gif|svg)$/i,
        type: 'asset/resource',
      },
      {
        test: /\.(jsx?|tsx?)$/,
        use: [
          {
            loader: 'builtin:swc-loader',
            options: {
              jsc: {
                parser: {
                  syntax: 'typescript',
                  tsx: true,
                },
                transform: {
                  react: {
                    runtime: 'automatic',
                    development: isDev,
                    refresh: isDev,
                  },
                },
              },
              env: { targets },
            },
          },
        ],
      },
    ],
  },
  devServer: {
    port: 1420,
  },
  plugins: [
    new rspack.HtmlRspackPlugin({
      template: './index.html',
    }),
    isDev ? new ReactRefreshRspackPlugin() : null,
    isDev ? reactDevToolsPlugin() : null,
  ].filter(Boolean),
  optimization: {
    minimizer: [
      new rspack.SwcJsMinimizerRspackPlugin(),
      new rspack.LightningCssMinimizerRspackPlugin({
        minimizerOptions: { targets },
      }),
    ],
  },
  experiments: {
    css: true,
    topLevelAwait: true,
  },
});
