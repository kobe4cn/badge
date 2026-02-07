# 通知推送工作服务
# 消费 Kafka 中的通知事件，负责向用户推送徽章授予、到期等通知消息

# ---- 构建阶段 ----
FROM rust:1.87-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev protobuf-compiler cmake build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY benches/ benches/

RUN cargo build --release -p notification-worker --bin notification-worker

# ---- 运行阶段 ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/notification-worker /app/notification-worker
COPY config/ /app/config/

ENV RUST_LOG=info

EXPOSE 50055

CMD ["./notification-worker"]
