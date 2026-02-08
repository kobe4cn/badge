# 徽章管理核心服务
# 提供 gRPC 接口，处理徽章的授予、撤销、兑换等核心业务逻辑

# ---- 构建阶段 ----
FROM rust:1.88-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libcurl4-openssl-dev protobuf-compiler cmake build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY benches/ benches/

RUN cargo build --release -p badge-management-service --bin badge-management

# ---- 运行阶段 ----
FROM debian:bookworm-slim

# gRPC 服务不需要 curl，健康检查由 docker-compose 通过 grpc_health_probe 配置
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/badge-management /app/badge-management
COPY config/ /app/config/

ENV RUST_LOG=info

EXPOSE 50052

CMD ["./badge-management"]
