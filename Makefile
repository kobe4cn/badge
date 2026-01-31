.PHONY: all setup build test clean dev-backend dev-backend-core dev-frontend infra-up infra-down mock-server mock-generate kafka-init kafka-topics e2e-test e2e-cascade e2e-redemption e2e-refund

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
	podman exec -i badge-postgres psql -U badge -d badge_db < migrations/20250203_001_schema_alignment.sql
	@echo "All migrations completed"

db-reset:
	@echo "Resetting database..."
	podman exec -i badge-postgres psql -U badge -d badge_db -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
	$(MAKE) db-migrate

# 初始化测试数据（执行测试前使用）
db-init-test:
	@echo "Initializing test data..."
	podman exec -i badge-postgres psql -U badge -d badge_db < scripts/init_test_data.sql
	@echo "Test data initialized"

# 完整数据库初始化（迁移 + 测试数据）
db-setup:
	$(MAKE) db-migrate
	$(MAKE) db-init-test

# 完整数据库重置（清理 + 迁移 + 测试数据）
db-reset-full:
	$(MAKE) db-reset
	$(MAKE) db-init-test

# =============================================
# E2E 测试
# =============================================

# 运行级联触发测试场景
e2e-cascade:
	@echo "Running cascade trigger E2E test..."
	@echo "Step 1: Grant first_checkin badge (id=2)"
	curl -s -X POST http://localhost:8080/api/admin/grants \
		-H "Content-Type: application/json" \
		-d '{"user_id": "e2e-cascade-$(shell date +%s)", "badge_id": 2, "source_type": "MANUAL"}' | jq .
	@echo ""
	@echo "Step 2: Grant social badge (id=6) - should trigger KOC cascade"
	curl -s -X POST http://localhost:8080/api/admin/grants \
		-H "Content-Type: application/json" \
		-d '{"user_id": "e2e-cascade-$(shell date +%s)", "badge_id": 6, "source_type": "MANUAL"}' | jq .
	@echo ""
	@echo "Step 3: Query user badges"
	curl -s "http://localhost:8080/api/admin/users/e2e-cascade-$(shell date +%s)/badges" | jq .

# 运行兑换测试场景
e2e-redemption:
	@echo "Running redemption E2E test..."
	@echo "This requires a user with KOC (8) and first_purchase (3) badges"
	@echo "Use: make e2e-redemption USER=<user_id>"
	@if [ -n "$(USER)" ]; then \
		echo "Redeeming park_star for user $(USER)..." && \
		curl -s -X POST "http://localhost:50052/badge.BadgeService/RedeemBadge" \
			-H "Content-Type: application/json" \
			-d '{"user_id": "$(USER)", "redemption_rule_id": 1}' | jq .; \
	fi

# 运行退款测试场景
e2e-refund:
	@echo "Running refund E2E test..."
	@echo "Usage: make e2e-refund USER=<user_id> BADGES='[3,4]'"
	@if [ -n "$(USER)" ] && [ -n "$(BADGES)" ]; then \
		echo "Sending refund event for user $(USER)..." && \
		echo '{"event_type": "refund", "user_id": "$(USER)", "order_id": "ORD-REFUND-$(shell date +%s)", "amount": 100.0, "badge_ids": $(BADGES)}' | \
		podman exec -i badge-kafka kafka-console-producer --bootstrap-server localhost:9092 --topic badge.transaction.events && \
		echo "Refund event sent. Check user badges status."; \
	else \
		echo "Example: make e2e-refund USER=test-user-001 BADGES='[3,4]'"; \
	fi

# 运行完整 E2E 测试套件
e2e-test:
	@echo "Running full E2E test suite..."
	@echo ""
	@echo "=== 1. Cascade Trigger Test ==="
	$(MAKE) e2e-cascade
	@echo ""
	@echo "=== 2. Redemption Test (manual) ==="
	@echo "Run: make e2e-redemption USER=<user_with_required_badges>"
	@echo ""
	@echo "=== 3. Refund Test (manual) ==="
	@echo "Run: make e2e-refund USER=<user_id> BADGES='[3,4]'"
	@echo ""
	@echo "E2E test suite completed. Some tests require manual user setup."

# 运行集成测试
test-integration:
	cargo test --workspace -- --ignored

# =============================================
# 代码检查
# =============================================
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
	@echo "  test-integration - Run integration tests (--ignored)"
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
	@echo "E2E Testing:"
	@echo "  e2e-test         - Run full E2E test suite"
	@echo "  e2e-cascade      - Test cascade trigger (首次签到+社交→KOC)"
	@echo "  e2e-redemption   - Test badge redemption (USER=<id>)"
	@echo "  e2e-refund       - Test refund flow (USER=<id> BADGES='[ids]')"
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
	@echo "  db-init-test     - Initialize test data only"
	@echo "  db-setup         - Full setup (migrate + test data)"
	@echo "  db-reset-full    - Full reset (clean + migrate + test data)"
	@echo ""
	@echo "Code Quality:"
	@echo "  lint             - Run linters"
	@echo "  fmt              - Format code"
	@echo "  fmt-check        - Check code formatting"
