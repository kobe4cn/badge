#!/bin/bash
set -e

echo "🚀 Setting up development environment..."

# 检查依赖
command -v podman >/dev/null 2>&1 || { echo "❌ Podman is required but not installed."; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "❌ Cargo is required but not installed."; exit 1; }
command -v pnpm >/dev/null 2>&1 || { echo "❌ pnpm is required but not installed."; exit 1; }

# 启动基础设施
echo "📦 Starting infrastructure..."
podman compose -f docker/docker-compose.infra.yml up -d

# 等待服务就绪
echo "⏳ Waiting for services to be ready..."
sleep 10

# 运行数据库迁移（执行全部迁移文件）
echo "🗃️ Running database migrations..."
for f in migrations/*.sql; do
  echo "  Applying $f..."
  podman exec -i badge-postgres psql -U badge -d badge_db < "$f" || true
done

# 安装前端依赖
echo "📦 Installing frontend dependencies..."
cd web/admin-ui && pnpm install && cd ../..

# 构建 Rust 项目
echo "🔨 Building Rust project..."
cargo build

echo "✅ Development environment is ready!"
echo ""
echo "Available commands:"
echo "  make dev-backend   - Start all backend services"
echo "  make dev-frontend  - Start frontend dev server"
echo "  make test          - Run all tests"
echo "  make infra-up      - Start infrastructure"
echo "  make infra-down    - Stop infrastructure"
