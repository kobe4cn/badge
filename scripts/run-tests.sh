#!/bin/bash
set -e

echo "🧪 Running tests..."

# Rust 测试
echo "📦 Running Rust tests..."
cargo test --workspace

# 前端测试
echo "📦 Running frontend lint..."
cd web/admin-ui && pnpm run lint && cd ../..

echo "✅ All tests passed!"
