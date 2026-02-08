# Badge 管理后台 HTTP 服务
# 提供徽章系统的 REST API，包括徽章、系列、分类的 CRUD 以及用户认证和权限管理

# ---- 构建阶段 ----
FROM rust:1.88-slim-bookworm AS builder

# rdkafka 的 cmake-build feature 需要 cmake，proto 编译需要 protobuf-compiler
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libcurl4-openssl-dev protobuf-compiler cmake build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# 复制整个 workspace，因为 crate 之间存在路径依赖
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY benches/ benches/

RUN cargo build --release -p badge-admin-service --bin badge-admin

# ---- 运行阶段 ----
FROM debian:bookworm-slim

# HTTP 服务需要 curl 做健康检查
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/badge-admin /app/badge-admin
COPY config/ /app/config/

ENV RUST_LOG=info

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["./badge-admin"]
