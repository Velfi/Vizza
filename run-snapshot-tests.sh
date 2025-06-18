#!/bin/bash

# Script to run snapshot tests for the simulations

echo "Running snapshot tests..."
echo "========================"

cd src-tauri

# Run tests with single thread to avoid GPU conflicts
echo "Running tests (single-threaded for GPU compatibility)..."
cargo test --test snapshot_tests -- --test-threads=1

# Check if tests passed
if [ $? -eq 0 ]; then
    echo ""
    echo "✅ All tests passed!"
    echo ""
    echo "To review snapshots, run:"
    echo "  cd src-tauri && cargo insta review"
else
    echo ""
    echo "❌ Some tests failed!"
    echo ""
    echo "To review snapshot differences, run:"
    echo "  cd src-tauri && cargo insta review"
fi