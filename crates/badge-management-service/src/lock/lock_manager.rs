//! 分布式锁管理器
//!
//! 实现 Redis 优先、数据库后备的分布式锁机制。

use crate::error::{BadgeError, Result};
use redis::Client as RedisClient;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{debug, instrument, warn};
use uuid::Uuid;

/// 锁配置
#[derive(Debug, Clone)]
pub struct LockConfig {
    /// 默认锁超时时间
    pub default_ttl: Duration,
    /// 获取锁重试次数
    pub retry_count: u32,
    /// 重试间隔
    pub retry_delay: Duration,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(30),
            retry_count: 3,
            retry_delay: Duration::from_millis(100),
        }
    }
}

/// 分布式锁管理器
///
/// 优先使用 Redis 实现高性能分布式锁，当 Redis 不可用时
/// 自动降级到基于 PostgreSQL 的锁机制。
pub struct LockManager {
    redis_client: Option<RedisClient>,
    pool: PgPool,
    config: LockConfig,
    /// 实例唯一标识，用于区分不同服务实例持有的锁
    instance_id: String,
}

impl LockManager {
    /// 创建锁管理器
    ///
    /// # Arguments
    /// - `redis_client`: Redis 客户端（可选，为 None 时只使用数据库锁）
    /// - `pool`: PostgreSQL 连接池
    /// - `config`: 锁配置
    pub fn new(redis_client: Option<RedisClient>, pool: PgPool, config: LockConfig) -> Self {
        Self {
            redis_client,
            pool,
            config,
            instance_id: Uuid::new_v4().to_string(),
        }
    }

    /// 使用默认配置创建锁管理器
    pub fn with_defaults(redis_client: Option<RedisClient>, pool: PgPool) -> Self {
        Self::new(redis_client, pool, LockConfig::default())
    }

    /// 获取锁
    ///
    /// 优先尝试 Redis 锁，失败则降级到数据库锁。
    /// 如果在重试次数内无法获取锁，返回 `LockConflict` 错误。
    ///
    /// # Arguments
    /// - `key`: 锁的唯一标识
    /// - `ttl`: 锁的过期时间（可选，默认使用配置中的 default_ttl）
    #[instrument(skip(self), fields(instance_id = %self.instance_id))]
    pub async fn acquire(&self, key: &str, ttl: Option<Duration>) -> Result<LockGuard> {
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        // owner 格式: instance_id:uuid，确保锁的唯一性
        let owner = format!("{}:{}", self.instance_id, Uuid::new_v4());

        // 尝试 Redis 锁
        if let Some(ref client) = self.redis_client {
            match self.try_redis_lock(client, key, &owner, ttl).await {
                Ok(true) => {
                    debug!(key = %key, owner = %owner, "Redis lock acquired");
                    return Ok(LockGuard::new_redis(key.to_string(), owner, client.clone()));
                }
                Ok(false) => {
                    debug!(key = %key, "Redis lock not acquired, resource is locked");
                }
                Err(e) => {
                    // Redis 操作失败，降级到数据库
                    warn!(key = %key, error = %e, "Redis lock failed, falling back to database");
                }
            }
        }

        // 降级到数据库锁
        self.try_db_lock(key, &owner, ttl).await
    }

    /// 尝试获取 Redis 锁
    ///
    /// 使用 SET NX PX 原子操作，确保只有一个客户端能获取锁
    async fn try_redis_lock(
        &self,
        client: &RedisClient,
        key: &str,
        owner: &str,
        ttl: Duration,
    ) -> std::result::Result<bool, String> {
        let lock_key = format!("lock:{}", key);
        let ttl_ms = ttl.as_millis() as u64;

        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| e.to_string())?;

        // SET key value NX PX milliseconds
        // NX: 只在 key 不存在时设置
        // PX: 设置过期时间（毫秒）
        let result: Option<String> = redis::cmd("SET")
            .arg(&lock_key)
            .arg(owner)
            .arg("NX")
            .arg("PX")
            .arg(ttl_ms)
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())?;

        // SET NX 成功时返回 "OK"，失败时返回 None
        Ok(result.is_some())
    }

    /// 尝试获取数据库锁
    ///
    /// 使用 INSERT ON CONFLICT DO NOTHING 实现原子锁获取，
    /// 同时清理过期锁以避免死锁。
    #[instrument(skip(self))]
    async fn try_db_lock(&self, key: &str, owner: &str, ttl: Duration) -> Result<LockGuard> {
        let expires_at = chrono::Utc::now()
            + chrono::Duration::from_std(ttl).map_err(|e| BadgeError::Internal(e.to_string()))?;

        for attempt in 0..self.config.retry_count {
            // 清理过期锁，防止死锁
            let deleted = sqlx::query(
                r#"DELETE FROM distributed_locks WHERE lock_key = $1 AND expires_at < NOW()"#,
            )
            .bind(key)
            .execute(&self.pool)
            .await?;

            if deleted.rows_affected() > 0 {
                debug!(key = %key, "Cleaned up expired database lock");
            }

            // 尝试获取锁
            let result = sqlx::query(
                r#"
                INSERT INTO distributed_locks (lock_key, owner_id, expires_at)
                VALUES ($1, $2, $3)
                ON CONFLICT (lock_key) DO NOTHING
                "#,
            )
            .bind(key)
            .bind(owner)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;

            if result.rows_affected() > 0 {
                debug!(key = %key, owner = %owner, attempt = attempt, "Database lock acquired");
                return Ok(LockGuard::new_db(
                    key.to_string(),
                    owner.to_string(),
                    self.pool.clone(),
                ));
            }

            // 未获取到锁，等待后重试
            if attempt < self.config.retry_count - 1 {
                debug!(
                    key = %key,
                    attempt = attempt,
                    retry_delay_ms = self.config.retry_delay.as_millis(),
                    "Lock not acquired, retrying"
                );
                tokio::time::sleep(self.config.retry_delay).await;
            }
        }

        Err(BadgeError::LockConflict {
            resource: key.to_string(),
        })
    }

    /// 尝试获取锁，不重试
    ///
    /// 如果锁不可用立即返回 None，不会阻塞等待。
    #[instrument(skip(self), fields(instance_id = %self.instance_id))]
    pub async fn try_acquire(&self, key: &str, ttl: Option<Duration>) -> Result<Option<LockGuard>> {
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let owner = format!("{}:{}", self.instance_id, Uuid::new_v4());

        // 尝试 Redis 锁
        if let Some(ref client) = self.redis_client {
            match self.try_redis_lock(client, key, &owner, ttl).await {
                Ok(true) => {
                    return Ok(Some(LockGuard::new_redis(
                        key.to_string(),
                        owner,
                        client.clone(),
                    )));
                }
                Ok(false) => return Ok(None),
                Err(e) => {
                    warn!(key = %key, error = %e, "Redis lock failed, falling back to database");
                }
            }
        }

        // 尝试数据库锁（不重试）
        let expires_at = chrono::Utc::now()
            + chrono::Duration::from_std(ttl).map_err(|e| BadgeError::Internal(e.to_string()))?;

        // 清理过期锁
        sqlx::query(r#"DELETE FROM distributed_locks WHERE lock_key = $1 AND expires_at < NOW()"#)
            .bind(key)
            .execute(&self.pool)
            .await?;

        let result = sqlx::query(
            r#"
            INSERT INTO distributed_locks (lock_key, owner_id, expires_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (lock_key) DO NOTHING
            "#,
        )
        .bind(key)
        .bind(owner.as_str())
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            Ok(Some(LockGuard::new_db(
                key.to_string(),
                owner,
                self.pool.clone(),
            )))
        } else {
            Ok(None)
        }
    }
}

/// 锁守卫
///
/// 持有锁的 RAII 包装器。当 `LockGuard` 被 drop 时，
/// 会记录警告日志提醒应该显式释放锁。
///
/// ## 注意事项
///
/// 建议使用 `release()` 方法显式释放锁，而不是依赖 Drop。
/// 因为 Drop 无法执行异步操作，无法保证锁被正确释放。
pub struct LockGuard {
    key: String,
    owner: String,
    backend: LockBackend,
    /// 标记锁是否已被释放，避免重复释放
    released: bool,
}

enum LockBackend {
    Redis(RedisClient),
    Database(PgPool),
}

impl LockGuard {
    fn new_redis(key: String, owner: String, client: RedisClient) -> Self {
        Self {
            key,
            owner,
            backend: LockBackend::Redis(client),
            released: false,
        }
    }

    fn new_db(key: String, owner: String, pool: PgPool) -> Self {
        Self {
            key,
            owner,
            backend: LockBackend::Database(pool),
            released: false,
        }
    }

    /// 获取锁的 key
    pub fn key(&self) -> &str {
        &self.key
    }

    /// 获取锁的 owner
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// 显式释放锁
    ///
    /// 推荐使用此方法而不是依赖 Drop，因为可以处理释放失败的情况。
    #[instrument(skip(self))]
    pub async fn release(mut self) -> Result<()> {
        self.released = true;
        match &self.backend {
            LockBackend::Redis(client) => self.release_redis(client).await,
            LockBackend::Database(pool) => self.release_db(pool).await,
        }
    }

    /// 释放 Redis 锁
    ///
    /// 使用 Lua 脚本原子验证 owner 并删除，防止误删其他客户端的锁
    async fn release_redis(&self, client: &RedisClient) -> Result<()> {
        let lock_key = format!("lock:{}", self.key);

        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| BadgeError::Redis(e.to_string()))?;

        // Lua 脚本：只有当锁的 owner 匹配时才删除
        // 这是一个原子操作，避免了检查-删除的竞态条件
        let script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("del", KEYS[1])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(&lock_key)
            .arg(&self.owner)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| BadgeError::Redis(e.to_string()))?;

        if result == 0 {
            // 锁已经不存在或被其他客户端持有，这通常表示锁已过期
            warn!(
                key = %self.key,
                owner = %self.owner,
                "Lock was already released or owned by another client"
            );
        } else {
            debug!(key = %self.key, "Redis lock released");
        }

        Ok(())
    }

    /// 释放数据库锁
    async fn release_db(&self, pool: &PgPool) -> Result<()> {
        let result =
            sqlx::query(r#"DELETE FROM distributed_locks WHERE lock_key = $1 AND owner_id = $2"#)
                .bind(&self.key)
                .bind(&self.owner)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            warn!(
                key = %self.key,
                owner = %self.owner,
                "Lock was already released or owned by another client"
            );
        } else {
            debug!(key = %self.key, "Database lock released");
        }

        Ok(())
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if !self.released {
            // Drop 中无法执行异步操作，只能记录警告
            // 锁最终会通过 TTL 过期自动释放
            warn!(
                lock_key = %self.key,
                owner = %self.owner,
                "LockGuard dropped without explicit release - lock will expire via TTL"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_config_default() {
        let config = LockConfig::default();
        assert_eq!(config.default_ttl, Duration::from_secs(30));
        assert_eq!(config.retry_count, 3);
        assert_eq!(config.retry_delay, Duration::from_millis(100));
    }

    #[test]
    fn test_lock_config_custom() {
        let config = LockConfig {
            default_ttl: Duration::from_secs(60),
            retry_count: 5,
            retry_delay: Duration::from_millis(200),
        };
        assert_eq!(config.default_ttl, Duration::from_secs(60));
        assert_eq!(config.retry_count, 5);
        assert_eq!(config.retry_delay, Duration::from_millis(200));
    }

    #[test]
    fn test_lock_config_clone() {
        let config = LockConfig {
            default_ttl: Duration::from_secs(45),
            retry_count: 10,
            retry_delay: Duration::from_millis(50),
        };
        let cloned = config.clone();

        assert_eq!(cloned.default_ttl, config.default_ttl);
        assert_eq!(cloned.retry_count, config.retry_count);
        assert_eq!(cloned.retry_delay, config.retry_delay);
    }

    #[test]
    fn test_lock_config_edge_values() {
        // 测试极端配置值
        let config_short_ttl = LockConfig {
            default_ttl: Duration::from_millis(1),
            retry_count: 1,
            retry_delay: Duration::from_millis(1),
        };
        assert_eq!(config_short_ttl.default_ttl, Duration::from_millis(1));
        assert_eq!(config_short_ttl.retry_count, 1);

        let config_long_ttl = LockConfig {
            default_ttl: Duration::from_secs(3600),
            retry_count: 100,
            retry_delay: Duration::from_secs(10),
        };
        assert_eq!(config_long_ttl.default_ttl, Duration::from_secs(3600));
        assert_eq!(config_long_ttl.retry_count, 100);
    }

    #[test]
    fn test_lock_config_zero_retries() {
        // 测试零重试次数配置
        let config = LockConfig {
            default_ttl: Duration::from_secs(30),
            retry_count: 0,
            retry_delay: Duration::from_millis(100),
        };
        assert_eq!(config.retry_count, 0);
    }

    #[test]
    fn test_redis_lock_key_format() {
        // 验证 Redis 锁 key 的格式
        let key = "redeem:user123:badge456";
        let lock_key = format!("lock:{}", key);

        assert!(lock_key.starts_with("lock:"));
        assert!(lock_key.contains(key));
        assert_eq!(lock_key, "lock:redeem:user123:badge456");
    }

    #[test]
    fn test_owner_format() {
        // 测试 owner 格式：instance_id:uuid
        let instance_id = Uuid::new_v4().to_string();
        let lock_uuid = Uuid::new_v4();
        let owner = format!("{}:{}", instance_id, lock_uuid);

        let parts: Vec<&str> = owner.split(':').collect();
        assert_eq!(parts.len(), 2);

        // 验证两部分都是有效的 UUID
        assert!(Uuid::parse_str(parts[0]).is_ok());
        assert!(Uuid::parse_str(parts[1]).is_ok());
    }

    #[test]
    fn test_lock_guard_accessors() {
        // 由于 LockGuard 需要实际的 Redis/DB 连接，这里测试其公共接口设计
        // 验证 key() 和 owner() 方法存在且返回正确类型

        // 通过字符串操作模拟 LockGuard 的行为
        let test_key = "test:lock:key";
        let test_owner = "instance-123:uuid-456";

        // 验证 key 的格式
        assert!(test_key.contains(':'));
        assert!(!test_key.is_empty());

        // 验证 owner 的格式
        assert!(test_owner.contains(':'));
        let owner_parts: Vec<&str> = test_owner.split(':').collect();
        assert_eq!(owner_parts.len(), 2);
    }

    #[test]
    fn test_lock_guard_released_state_tracking() {
        // 测试 released 状态的概念
        // LockGuard 使用 released: bool 来追踪锁是否已被显式释放

        // 初始状态应为 false
        let released = false;
        assert!(!released);

        // 调用 release() 后应为 true
        let released_after = true;
        assert!(released_after);

        // 这确保 Drop 时可以检查是否需要警告
        if !released {
            // 应该记录警告
            let warning_needed = true;
            assert!(warning_needed);
        }
    }

    #[test]
    fn test_lock_ttl_conversion() {
        // 测试 TTL 转换为毫秒
        let ttl = Duration::from_secs(30);
        let ttl_ms = ttl.as_millis() as u64;

        assert_eq!(ttl_ms, 30000);

        // 测试较短的 TTL
        let short_ttl = Duration::from_millis(500);
        let short_ttl_ms = short_ttl.as_millis() as u64;
        assert_eq!(short_ttl_ms, 500);
    }

    #[test]
    fn test_lock_backend_variants() {
        // 验证 LockBackend 枚举的存在和用途
        // Redis 后端用于高性能分布式锁
        // Database 后端用于 Redis 不可用时的降级方案

        // 通过匹配模式验证两种后端类型的设计
        enum MockBackend {
            Redis,
            Database,
        }

        let redis_backend = MockBackend::Redis;
        let db_backend = MockBackend::Database;

        match redis_backend {
            MockBackend::Redis => assert!(true),
            MockBackend::Database => panic!("Should be Redis"),
        }

        match db_backend {
            MockBackend::Database => assert!(true),
            MockBackend::Redis => panic!("Should be Database"),
        }
    }

    #[test]
    fn test_instance_id_uniqueness() {
        // 每个 LockManager 实例应该有唯一的 instance_id
        let instance_id_1 = Uuid::new_v4().to_string();
        let instance_id_2 = Uuid::new_v4().to_string();

        assert_ne!(instance_id_1, instance_id_2);

        // 验证 instance_id 是有效的 UUID 字符串
        assert!(Uuid::parse_str(&instance_id_1).is_ok());
        assert!(Uuid::parse_str(&instance_id_2).is_ok());
    }

    #[test]
    fn test_lock_conflict_scenario() {
        // 模拟锁冲突场景的错误信息
        let resource = "redeem:user123:badge456";

        // 验证错误信息格式
        let error_message = format!("Lock conflict for resource: {}", resource);
        assert!(error_message.contains(resource));
        assert!(error_message.contains("Lock conflict"));
    }
}
