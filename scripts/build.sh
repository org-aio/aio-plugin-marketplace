#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

target_dir=${CARGO_TARGET_DIR:-target}
dx_bin=${DX:-dx}
# 前后端都从当前源码构建，整包只依赖公开 WIT 和浏览器 SDK。
rm -rf dist/frontend "$target_dir/dx/aio-plugin-marketplace-client/release/web/public"
"$dx_bin" build --package aio-plugin-marketplace-client --web --release --inject-loading-scripts false --debug-symbols false
node scripts/package-frontend.mjs "$target_dir/dx/aio-plugin-marketplace-client/release/web/public"
cargo build --locked --release -p aio-plugin-marketplace-server --target wasm32-wasip2
cp "$target_dir/wasm32-wasip2/release/aio_plugin_marketplace_server.wasm" dist/marketplace.wasm
