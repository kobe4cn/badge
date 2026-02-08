# 统一规则引擎服务
# 提供 gRPC 接口，负责徽章授予规则的评估与匹配，支持动态规则模板

# ---- 构建阶段 ----
FROM rust:1.88-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libcurl4-openssl-dev protobuf-compiler cmake build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY benches/ benches/

RUN cargo build --release -p unified-rule-engine --bin rule-engine

# ---- 运行阶段 ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/rule-engine /app/rule-engine
COPY config/ /app/config/

ENV RUST_LOG=info

EXPOSE 50051

CMD ["./rule-engine"]
