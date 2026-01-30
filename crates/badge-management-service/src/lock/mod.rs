//! 分布式锁模块
//!
//! 提供 Redis 优先、数据库后备的分布式锁实现。
//!
//! ## 设计理念
//!
//! - **Redis 优先**: 高性能场景下优先使用 Redis 分布式锁
//! - **数据库后备**: Redis 不可用时自动降级到基于 PostgreSQL 的锁
//! - **RAII 模式**: 通过 `LockGuard` 确保锁的自动释放
//!
//! ## 使用示例
//!
//! ```ignore
//! let lock_manager = LockManager::new(cache, pool, LockConfig::default());
//!
//! // 获取锁
//! let guard = lock_manager.acquire("resource:123", None).await?;
//!
//! // 执行受保护的操作
//! do_critical_work().await?;
//!
//! // 显式释放锁（推荐）
//! guard.release().await?;
//! ```

mod lock_manager;

pub use lock_manager::{LockConfig, LockGuard, LockManager};
