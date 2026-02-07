# Mock 服务
# 在开发和测试环境中模拟外部依赖（如用户服务、支付服务），避免依赖真实第三方接口

# ---- 构建阶段 ----
FROM rust:1.87-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev protobuf-compiler cmake build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY benches/ benches/

RUN cargo build --release -p mock-services --bin mock-server

# ---- 运行阶段 ----
FROM debian:bookworm-slim

# HTTP 服务需要 curl 做健康检查
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/mock-server /app/mock-server
COPY config/ /app/config/

ENV RUST_LOG=info

EXPOSE 8090

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8090/health || exit 1

CMD ["./mock-server"]
