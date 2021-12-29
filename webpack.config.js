const HtmlWebpackPlugin = require('html-webpack-plugin');

module.exports = {
  entry: "./client/src/index.tsx",

  output: {
    filename: "bundle.js",
    path: __dirname + "/target/client/"
  },

  devtool: "source-map",

  resolve: {
    extensions: [".ts", ".tsx", ".js", ".json"]
  },

  module: {
    rules: [
      { test: /\.tsx?$/, loader: "ts-loader" },
      { enforce: "pre", test: /\.js$/, loader: "source-map-loader" }
    ],
  },

  plugins: [
    new HtmlWebpackPlugin({
      "template": "client/src/index.html"
    })
  ],

  devServer: {
    static: {
      directory: "client/webroot/"
    },
    proxy: {
      "/api/ws": {
        target: "ws://0.0.0.0:8081",
        ws: true
      },
      "/api": {
        target: "http://0.0.0.0:8081"
      }
    }
  }
};
