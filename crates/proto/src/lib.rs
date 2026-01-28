//! gRPC/Protobuf 定义

pub mod rule_engine {
    include!("generated/badge.rule_engine.rs");
}

pub mod badge {
    include!("generated/badge.management.rs");
}
