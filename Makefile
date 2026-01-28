.PHONY: all setup build test clean dev-backend dev-frontend infra-up infra-down

# 默认目标
all: build

# 开发环境设置
setup:
	chmod +x scripts/*.sh
	./scripts/dev-setup.sh

# 构建
build:
	cargo build --workspace

build-release:
	cargo build --workspace --release

# 测试
test:
	cargo test --workspace

test-verbose:
	cargo test --workspace -- --nocapture

# 清理
clean:
	cargo clean
	rm -rf web/admin-ui/dist
	rm -rf web/admin-ui/node_modules

# 启动后端开发服务
dev-backend:
	@echo "Starting backend services..."
	cargo run --bin rule-engine &
	cargo run --bin badge-management &
	cargo run --bin badge-admin &
	@echo "Backend services started"

# 启动前端开发服务
dev-frontend:
	cd web/admin-ui && pnpm run dev

# 基础设施管理
infra-up:
	docker compose -f docker/docker-compose.infra.yml up -d

infra-down:
	docker compose -f docker/docker-compose.infra.yml down

infra-logs:
	docker compose -f docker/docker-compose.infra.yml logs -f

infra-restart:
	docker compose -f docker/docker-compose.infra.yml restart

# 数据库迁移
db-migrate:
	docker exec -i badge-postgres psql -U badge -d badge_db < migrations/20250128_001_init_schema.sql

db-reset:
	docker exec -i badge-postgres psql -U badge -d badge_db -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
	docker exec -i badge-postgres psql -U badge -d badge_db < migrations/20250128_001_init_schema.sql

# 代码检查
lint:
	cargo clippy --workspace -- -D warnings
	cd web/admin-ui && pnpm run lint

# 格式化
fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

# Proto 生成（后续使用）
proto:
	cd crates/proto && cargo build

# 帮助
help:
	@echo "Available targets:"
	@echo "  setup          - Set up development environment"
	@echo "  build          - Build all Rust crates"
	@echo "  build-release  - Build release version"
	@echo "  test           - Run all tests"
	@echo "  clean          - Clean build artifacts"
	@echo "  dev-backend    - Start backend services"
	@echo "  dev-frontend   - Start frontend dev server"
	@echo "  infra-up       - Start infrastructure (Docker)"
	@echo "  infra-down     - Stop infrastructure"
	@echo "  infra-logs     - View infrastructure logs"
	@echo "  db-migrate     - Run database migrations"
	@echo "  db-reset       - Reset database and run migrations"
	@echo "  lint           - Run linters"
	@echo "  fmt            - Format code"
