#!/bin/bash

# Install wasm-pack if not already installed
if ! command -v wasm-pack &> /dev/null; then
    cargo install wasm-pack
fi

# Build the project for WebAssembly
wasm-pack build --target web

# Create a simple server to serve the files
echo "Build complete! You can now serve the files using a local server."
echo "For example, using Python:"
echo "python3 -m http.server" 