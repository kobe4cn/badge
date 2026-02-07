# 交易事件消费服务
# 消费 Kafka 中的交易事件（如消费、充值），触发规则引擎评估是否授予徽章

# ---- 构建阶段 ----
FROM rust:1.87-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libcurl4-openssl-dev protobuf-compiler cmake build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY benches/ benches/

RUN cargo build --release -p event-transaction-service --bin event-transaction

# ---- 运行阶段 ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/event-transaction /app/event-transaction
COPY config/ /app/config/

ENV RUST_LOG=info

EXPOSE 50054

CMD ["./event-transaction"]
