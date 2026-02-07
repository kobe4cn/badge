# 用户互动事件消费服务
# 消费 Kafka 中的用户互动事件（如签到、点赞），触发规则引擎评估是否授予徽章

# ---- 构建阶段 ----
FROM rust:1.87-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev protobuf-compiler cmake build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY benches/ benches/

RUN cargo build --release -p event-engagement-service --bin event-engagement

# ---- 运行阶段 ----
FROM debian:bookworm-slim

# Kafka consumer 不对外提供 HTTP 接口，不需要 curl
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/event-engagement /app/event-engagement
COPY config/ /app/config/

ENV RUST_LOG=info

EXPOSE 50053

CMD ["./event-engagement"]
