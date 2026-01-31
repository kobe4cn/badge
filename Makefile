.PHONY: all setup build test clean dev-backend dev-backend-core dev-frontend infra-up infra-down mock-server mock-generate kafka-init kafka-topics

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

# 启动完整后端服务（6 个服务 + 与生产一致的事件处理流程）
dev-backend:
	@echo "Starting all backend services..."
	@echo "  - rule-engine (gRPC :50051)"
	@echo "  - badge-management (gRPC :50052)"
	@echo "  - badge-admin (HTTP :8080)"
	@echo "  - event-engagement (Kafka consumer :50053)"
	@echo "  - event-transaction (Kafka consumer :50054)"
	@echo "  - notification-worker (Kafka consumer :50055)"
	cargo run -p unified-rule-engine --bin rule-engine &
	cargo run -p badge-management-service --bin badge-management &
	cargo run -p badge-admin-service --bin badge-admin &
	cargo run -p event-engagement-service --bin event-engagement &
	cargo run -p event-transaction-service --bin event-transaction &
	cargo run -p notification-worker --bin notification-worker &
	@echo "All backend services started"

# 仅启动核心服务（不含 Kafka 事件消费者）
dev-backend-core:
	@echo "Starting core backend services only..."
	cargo run -p unified-rule-engine --bin rule-engine &
	cargo run -p badge-management-service --bin badge-management &
	cargo run -p badge-admin-service --bin badge-admin &
	@echo "Core backend services started"

# 启动前端开发服务
dev-frontend:
	cd web/admin-ui && pnpm run dev

# Mock 服务：启动 HTTP 服务器并生成测试数据
mock-server:
	@echo "Starting mock server with test data..."
	cargo run --bin mock-services -- server --port 3030 --populate --user-count 100

# Mock 服务：生成模拟事件到 Kafka
mock-generate:
	@echo "Usage: make mock-generate TYPE=<event_type> [USER=<user_id>] [COUNT=<n>]"
	@echo "Event types: sign_in, browse, share, like, purchase, refund, cancel"
	@echo "Example: make mock-generate TYPE=purchase USER=test_user COUNT=5"
	@if [ -n "$(TYPE)" ]; then \
		cargo run --bin mock-services -- generate \
			--event-type $(TYPE) \
			--user-id $${USER:-test_user} \
			--count $${COUNT:-1}; \
	fi

# Mock 服务：运行预定义场景
mock-scenario:
	@echo "Usage: make mock-scenario NAME=<scenario> [USER=<user_id>]"
	@echo "Scenarios: first_purchase, vip_upgrade, daily_check_in"
	@if [ -n "$(NAME)" ]; then \
		cargo run --bin mock-services -- scenario \
			--name $(NAME) \
			--user-id $${USER:-test_user}; \
	fi

# 基础设施管理
infra-up:
	podman compose -f docker/docker-compose.infra.yml up -d

infra-down:
	podman compose -f docker/docker-compose.infra.yml down

infra-logs:
	podman compose -f docker/docker-compose.infra.yml logs -f

infra-restart:
	podman compose -f docker/docker-compose.infra.yml restart

# Kafka topic 初始化（服务启动前必须执行）
kafka-init:
	@echo "Creating Kafka topics..."
	@podman exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --create --if-not-exists --topic badge.engagement.events --partitions 3 --replication-factor 1
	@podman exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --create --if-not-exists --topic badge.transaction.events --partitions 3 --replication-factor 1
	@podman exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --create --if-not-exists --topic badge.notifications --partitions 3 --replication-factor 1
	@podman exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --create --if-not-exists --topic badge.dlq --partitions 1 --replication-factor 1
	@echo "Kafka topics created successfully"
	@$(MAKE) kafka-topics

kafka-topics:
	@echo "Listing Kafka topics:"
	@podman exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --list | grep -E "^badge\." || echo "No badge topics found"

kafka-describe:
	@echo "=== badge.engagement.events ==="
	@podman exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --describe --topic badge.engagement.events
	@echo ""
	@echo "=== badge.transaction.events ==="
	@podman exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --describe --topic badge.transaction.events
	@echo ""
	@echo "=== badge.notifications ==="
	@podman exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --describe --topic badge.notifications
	@echo ""
	@echo "=== badge.dlq ==="
	@podman exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --describe --topic badge.dlq

# 数据库迁移（按顺序执行所有迁移文件）
db-migrate:
	@echo "Running database migrations..."
	podman exec -i badge-postgres psql -U badge -d badge_db < migrations/20250128_001_init_schema.sql
	podman exec -i badge-postgres psql -U badge -d badge_db < migrations/20250130_001_badge_dependency.sql
	podman exec -i badge-postgres psql -U badge -d badge_db < migrations/20250131_001_cascade_log.sql
	podman exec -i badge-postgres psql -U badge -d badge_db < migrations/20250201_001_user_badge_logs.sql
	podman exec -i badge-postgres psql -U badge -d badge_db < migrations/20250202_001_dynamic_rules.sql
	@echo "All migrations completed"

db-reset:
	@echo "Resetting database..."
	podman exec -i badge-postgres psql -U badge -d badge_db -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
	$(MAKE) db-migrate

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
	@echo ""
	@echo "Setup & Build:"
	@echo "  setup            - Set up development environment"
	@echo "  build            - Build all Rust crates"
	@echo "  build-release    - Build release version"
	@echo "  test             - Run all tests"
	@echo "  clean            - Clean build artifacts"
	@echo ""
	@echo "Development:"
	@echo "  dev-backend      - Start ALL 6 backend services (full event processing)"
	@echo "  dev-backend-core - Start only 3 core services (no Kafka consumers)"
	@echo "  dev-frontend     - Start frontend dev server"
	@echo ""
	@echo "Mock Services:"
	@echo "  mock-server      - Start mock HTTP server with test data"
	@echo "  mock-generate    - Generate events (TYPE=<type> [USER=<id>] [COUNT=<n>])"
	@echo "  mock-scenario    - Run predefined scenario (NAME=<name> [USER=<id>])"
	@echo ""
	@echo "Infrastructure:"
	@echo "  infra-up         - Start infrastructure (Podman)"
	@echo "  infra-down       - Stop infrastructure"
	@echo "  infra-logs       - View infrastructure logs"
	@echo "  infra-restart    - Restart infrastructure"
	@echo ""
	@echo "Kafka:"
	@echo "  kafka-init       - Create all required Kafka topics (run after infra-up)"
	@echo "  kafka-topics     - List all badge.* topics"
	@echo "  kafka-describe   - Show topic details (partitions, replicas)"
	@echo ""
	@echo "Database:"
	@echo "  db-migrate       - Run all database migrations"
	@echo "  db-reset         - Reset database and run migrations"
	@echo ""
	@echo "Code Quality:"
	@echo "  lint             - Run linters"
	@echo "  fmt              - Format code"
	@echo "  fmt-check        - Check code formatting"
