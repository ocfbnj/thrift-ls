# thrift-ls

A Thrift language server.

This project contains two programs:

- Rust binary: A language server for Thrift.
- VS Code extension: A VS Code extension for Thrift. This extension uses the WASM output of the Rust library.

## Features

- semantic syntax highlighting.
- go to definition.
- diagnostics.

## How to Build

### Prerequisites

- Cargo installed (see [here](https://rustup.rs/)).
- Node.js installed (optional, for building the VS Code extension).

### Build with Cargo

1. Clone and Build
    ~~~bash
    git clone https://github.com/ocfbnj/thrift-ls.git
    cd thrift-ls
    cargo build --release
    ~~~
    Now you can find the binary in `./target/release/thrift-ls`.

### Build VS Code Extension

1. Install wasm-pack and wasm-bindgen-cli

    ~~~bash
    cargo install wasm-pack wasm-bindgen-cli
    ~~~

2. Install vsce
    ~~~bash
    npm install -g @vscode/vsce
    ~~~

3. Build the VS Code extension

    ~~~bash
    cd editors/code
    npm install
    npm run compile
    vsce package
    ~~~

    Now you can find the VS Code extension in `./thrift-ls-x.x.x.vsix`.
