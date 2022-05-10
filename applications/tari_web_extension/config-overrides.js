// Copyright 2022 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

const path = require("path");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const KEY_MANAGER_PATH = "../../base_layer/key_manager/";
const { override, addWebpackPlugin } = require("customize-cra");
const webpack = require("webpack");

const wasmPlugin = new WasmPackPlugin({
  crateDirectory: path.resolve(__dirname, KEY_MANAGER_PATH),
  watchDirectories: [path.resolve(__dirname, KEY_MANAGER_PATH)],
  outDir: path.resolve(__dirname, "src/key_manager/"),
  extraArgs: "-- --features js",
});

module.exports = override(addWebpackPlugin(wasmPlugin), (config) => {
  config.resolve.extensions.push(".wasm");

  config.module.rules.forEach((rule) => {
    (rule.oneOf || []).forEach((oneOf) => {
      if (oneOf.loader && oneOf.loader.indexOf("file-loader") >= 0) {
        // Make file-loader ignore WASM files
        oneOf.exclude.push(/\.wasm$/);
      }
    });
  });

  config.resolve.fallback = {
    ...config.resolve.fallback,
    // stream: require.resolve("stream-browserify"),
    buffer: require.resolve("buffer"),
  }
  config.resolve.extensions = [...config.resolve.extensions, ".ts", ".js"]
  config.plugins = [
    ...config.plugins,
    new webpack.ProvidePlugin({
      process: "process/browser",
      Buffer: ["buffer", "Buffer"],
    }),
  ]

  return config;
});
