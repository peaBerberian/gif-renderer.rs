# Web demo

This directory contains a demo of `gif-renderer.rs` running as WebAssembly in a
browser.

It can be tested here: https://peaberberian.github.io/gif-renderer.rs/

## Build

From this directory:

```sh
npm install
npm run build
```

This will:

1. Build the Rust library as `wasm32-unknown-unknown`
2. Copy the generated `.wasm` artifact to `dist/pkg/`
3. Copy static web assets such as `index.html` and `favicon.svg` into `dist/`
4. Bundle the TypeScript frontend into `dist/app.js`

To preview locally:

```sh
npx http-server dist -p 8000
```
