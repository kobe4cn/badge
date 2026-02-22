//! Redis 缓存管理模块
//!
//! 提供 Redis 连接管理和常用缓存操作封装。

use crate::config::RedisConfig;
use crate::error::{BadgeError, Result};
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;
use tracing::{info, instrument};

/// Redis 缓存客户端
#[derive(Clone)]
pub struct Cache {
    client: Client,
}

impl Cache {
    /// 创建 Redis 客户端
    pub fn new(config: &RedisConfig) -> Result<Self> {
        let client = Client::open(config.url.as_str())?;
        info!("Redis client created");
        Ok(Self { client })
    }

    /// 获取连接
    async fn get_conn(&self) -> Result<MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(BadgeError::from)
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<()> {
        let mut conn = self.get_conn().await?;
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map(|_| ())
            .map_err(BadgeError::from)
    }

    /// 获取值
    #[instrument(skip(self))]
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.get_conn().await?;
        let value: Option<String> = conn.get(key).await?;

        match value {
            Some(v) => {
                let parsed: T = serde_json::from_str(&v).map_err(|e| {
                    BadgeError::Internal(format!("Cache deserialization error: {}", e))
                })?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// 设置值
    #[instrument(skip(self, value))]
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let serialized = serde_json::to_string(value)
            .map_err(|e| BadgeError::Internal(format!("Cache serialization error: {}", e)))?;

        let _: () = conn.set_ex(key, serialized, ttl.as_secs()).await?;
        Ok(())
    }

    /// 删除值
    #[instrument(skip(self))]
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let _: () = conn.del(key).await?;
        Ok(())
    }

    /// 批量删除（按模式）
    #[instrument(skip(self))]
    pub async fn delete_pattern(&self, pattern: &str) -> Result<u64> {
        let mut conn = self.get_conn().await?;
        let keys: Vec<String> = conn.keys(pattern).await?;

        if keys.is_empty() {
            return Ok(0);
        }

        let count: u64 = conn.del(keys).await?;
        Ok(count)
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_conn().await?;
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    /// 原子性地仅在 key 不存在时设置值，并指定 TTL
    ///
    /// 基于 Redis SET NX EX 实现，适用于分布式幂等检查和互斥控制。
    /// 返回 true 表示设置成功（key 不存在），false 表示 key 已存在。
    pub async fn set_nx<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<bool> {
        let mut conn = self.get_conn().await?;
        let serialized = serde_json::to_string(value)
            .map_err(|e| BadgeError::Internal(format!("Cache serialization error: {}", e)))?;

        let result: Option<String> = redis::cmd("SET")
            .arg(key)
            .arg(serialized)
            .arg("NX")
            .arg("EX")
            .arg(ttl.as_secs())
            .query_async(&mut conn)
            .await?;

        Ok(result.is_some())
    }

    /// 获取或设置（缓存穿透保护）
    #[instrument(skip(self, loader))]
    pub async fn get_or_set<T, F, Fut>(&self, key: &str, ttl: Duration, loader: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // 尝试从缓存获取
        if let Some(cached) = self.get::<T>(key).await? {
            return Ok(cached);
        }

        // 从数据源加载
        let value = loader().await?;

        // 写入缓存
        self.set(key, &value, ttl).await?;

        Ok(value)
    }

    /// 增量操作
    pub async fn incr(&self, key: &str, delta: i64) -> Result<i64> {
        let mut conn = self.get_conn().await?;
        let result: i64 = conn.incr(key, delta).await?;
        Ok(result)
    }

    /// 设置过期时间
    pub async fn expire(&self, key: &str, ttl: Duration) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let _: () = conn.expire(key, ttl.as_secs() as i64).await?;
        Ok(())
    }
}

/// 缓存键生成器
pub struct CacheKey;

impl CacheKey {
    pub fn user_badges(user_id: &str) -> String {
        format!("user:badge:{}", user_id)
    }

    pub fn badge_detail(badge_id: &str) -> String {
        format!("badge:detail:{}", badge_id)
    }

    pub fn badge_config(badge_id: &str) -> String {
        format!("badge:config:{}", badge_id)
    }

    pub fn user_badge_count(user_id: &str) -> String {
        format!("user:badge:count:{}", user_id)
    }

    pub fn rule(rule_id: &str) -> String {
        format!("rule:{}", rule_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(CacheKey::user_badges("123"), "user:badge:123");
        assert_eq!(CacheKey::badge_detail("abc"), "badge:detail:abc");
    }
}
