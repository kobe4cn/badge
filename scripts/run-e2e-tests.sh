#!/bin/bash
# E2E 测试运行脚本

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}=== 徽章系统 E2E 测试 ===${NC}"

# 解析参数
TEST_TYPE=${1:-all}
SKIP_CLEANUP=${2:-false}

# 启动基础设施
start_infra() {
    echo -e "${YELLOW}启动测试基础设施...${NC}"
    docker-compose -f docker-compose.test.yml up -d postgres redis kafka

    echo "等待服务就绪..."
    sleep 10

    # 运行数据库迁移
    echo -e "${YELLOW}运行数据库迁移...${NC}"
    sqlx database create || true
    sqlx migrate run
}

# 启动完整服务
start_services() {
    echo -e "${YELLOW}启动应用服务...${NC}"
    docker-compose -f docker-compose.test.yml --profile full up -d

    echo "等待服务就绪..."
    sleep 30
}

# 运行后端测试
run_backend_tests() {
    echo -e "${YELLOW}运行后端 E2E 测试...${NC}"
    cargo test --test e2e -- --ignored --test-threads=1 --nocapture
}

# 运行前端测试
run_frontend_tests() {
    echo -e "${YELLOW}运行前端 E2E 测试...${NC}"
    cd web/admin-ui
    npm ci
    npx playwright install --with-deps
    npx playwright test
    cd ../..
}

# 运行性能测试
run_performance_tests() {
    echo -e "${YELLOW}运行性能测试...${NC}"
    cargo test --test performance -- --ignored --test-threads=1 --nocapture
}

# 清理
cleanup() {
    if [ "$SKIP_CLEANUP" != "true" ]; then
        echo -e "${YELLOW}清理测试环境...${NC}"
        docker-compose -f docker-compose.test.yml down -v
    fi
}

# 主流程
main() {
    trap cleanup EXIT

    start_infra

    case $TEST_TYPE in
        backend)
            start_services
            run_backend_tests
            ;;
        frontend)
            start_services
            run_frontend_tests
            ;;
        performance)
            start_services
            run_performance_tests
            ;;
        all)
            start_services
            run_backend_tests
            run_frontend_tests
            ;;
        *)
            echo -e "${RED}未知测试类型: $TEST_TYPE${NC}"
            echo "用法: $0 [backend|frontend|performance|all] [skip_cleanup]"
            exit 1
            ;;
    esac

    echo -e "${GREEN}=== 测试完成 ===${NC}"
}

main
