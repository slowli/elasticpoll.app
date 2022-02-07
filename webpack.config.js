const { mkdir } = require('fs');
const path = require('path');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const AutoprefixerPlugin = require('autoprefixer');
const PrerenderSPAPlugin = require('prerender-spa-plugin');
const { InjectManifest } = require('workbox-webpack-plugin');

const Renderer = PrerenderSPAPlugin.PuppeteerRenderer;
const distPath = path.resolve(__dirname, 'dist');

// Monkey-patches `mkdirp` function to `compiler.outputFileSystem`, which is used
// by `PrerenderSPAPlugin`.
class MkdirpProviderPlugin {
  // eslint-disable-next-line class-methods-use-this
  apply(compiler) {
    // eslint-disable-next-line no-param-reassign
    compiler.outputFileSystem.mkdirp = (dirPath, options, callback) => {
      mkdir(dirPath, { ...(options || {}), recursive: true }, callback);
    };
  }
}

const config = {
  entry: { index: './webpack/index.js' },
  output: {
    path: distPath,
    publicPath: process.env.WEBPACK_PUBLIC_PATH || '/',
    filename: '_assets/js/[name].js',
    chunkFilename: '_assets/js/[name].[chunkhash:8].js',
    webassemblyModuleFilename: '_assets/js/[hash].module.wasm',
  },
  experiments: {
    asyncWebAssembly: true,
  },
  module: {
    rules: [
      {
        test: /\.js/,
        exclude: /node_modules/,
        use: 'babel-loader',
      },
      {
        test: /\.css$/i,
        use: [MiniCssExtractPlugin.loader, 'css-loader'],
      },
      {
        test: /\.scss$/i,
        use: [
          MiniCssExtractPlugin.loader,
          'css-loader',
          {
            loader: 'postcss-loader',
            options: {
              postcssOptions: {
                plugins: [AutoprefixerPlugin],
              },
            },
          },
          'sass-loader',
        ],
      },
      {
        test: /\.(woff|woff2)$/i,
        type: 'asset',
      },
    ],
  },
  optimization: {
    splitChunks: {
      chunks: 'all',
      cacheGroups: {
        vendors: false, // disable splitting the main chunk into 3rd-party and built-in parts
      },
    },
  },
  plugins: [
    new CopyWebpackPlugin({
      patterns: [{
        from: './webpack/favicon',
        to: '_assets/css/[name][ext]',
      }],
    }),
    new MiniCssExtractPlugin({
      filename: '_assets/css/[name].css',
    }),
    new WasmPackPlugin({
      crateDirectory: '.',
      extraArgs: '--no-typescript',
    }),
    new HtmlWebpackPlugin({
      filename: 'index.html',
      chunks: ['index'],
      template: 'webpack/index.html',
    }),
  ],
};

module.exports = (env, argv) => {
  const serviceWorkerExcludes = (argv.mode === 'development')
    ? [/.*/] // exclude precaching for dev builds (leads to infinite reloading loops)
    : [];
  config.plugins.push(new InjectManifest({
    swSrc: './webpack/service-worker.js',
    swDest: 'service-worker.js',
    exclude: serviceWorkerExcludes,
  }));

  if (argv.mode === 'production') {
    config.plugins.push(
      new MkdirpProviderPlugin(),
      new PrerenderSPAPlugin({
        staticDir: path.join(__dirname, 'dist'),
        routes: ['/', '/about', '/implementation'],
        renderer: new Renderer({
          injectProperty: '__PRERENDER__',
          inject: {}, // we don't need any properties, just for the global var to be set
          renderAfterDocumentEvent: 'wasm-rendered',
        }),
      }),
    );
  } else if (argv.mode === 'development') {
    config.devServer = {
      historyApiFallback: true,
    };
  }

  return config;
};
