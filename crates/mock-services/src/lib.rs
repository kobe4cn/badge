//! Mock Services
//!
//! 模拟外部服务的 crate，用于开发和测试环境。
//!
//! # 主要模块
//!
//! - `models`: 模拟数据模型（订单、用户、优惠券）
//! - `store`: 内存存储实现
//! - `generators`: 测试数据生成器
//!
//! # 使用示例
//!
//! ```rust
//! use mock_services::generators::{DataGenerator, GeneratorConfig};
//! use mock_services::models::{MockUser, MockOrder, MockCoupon};
//! use mock_services::store::MemoryStore;
//!
//! // 创建存储
//! let users: MemoryStore<MockUser> = MemoryStore::new();
//! let orders: MemoryStore<MockOrder> = MemoryStore::new();
//! let coupons: MemoryStore<MockCoupon> = MemoryStore::new();
//!
//! // 配置并生成数据
//! let config = GeneratorConfig {
//!     user_count: 50,
//!     orders_per_user: 1..5,
//!     coupons_per_user: 0..3,
//! };
//! let generator = DataGenerator::new(config);
//! generator.populate_stores(&users, &orders, &coupons);
//! ```

pub mod event_generator;
pub mod generators;
pub mod models;
pub mod scenarios;
pub mod services;
pub mod store;
