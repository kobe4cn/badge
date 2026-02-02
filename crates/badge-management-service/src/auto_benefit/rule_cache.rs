//! 自动权益规则缓存
//!
//! 实现按触发徽章索引的规则缓存，避免每次自动权益评估都查询数据库。
//!
//! ## 设计思路
//!
//! 用户获得徽章时，需要查询"哪些自动权益规则的 required_badges 包含该徽章"。
//! 如果每次都查数据库，会造成大量重复查询。此缓存将所有 auto_redeem=true 的规则
//! 加载到内存，并按触发徽章 ID 建立索引，实现 O(1) 查询。
//!
//! ## 缓存刷新策略
//!
//! 采用 TTL 机制自动刷新（默认 5 分钟），同时提供手动失效接口供规则变更时调用。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::models::{FrequencyConfig, RequiredBadge};

/// 缓存的规则信息
///
/// 只保留自动权益评估所需的字段，避免持有完整规则对象
#[derive(Debug, Clone)]
pub struct CachedRule {
    /// 规则 ID
    pub rule_id: i64,
    /// 规则名称
    pub name: String,
    /// 需要的徽章列表（包含数量要求）
    pub required_badges: Vec<RequiredBadge>,
    /// 关联的权益 ID
    pub benefit_id: i64,
    /// 频率限制配置
    pub frequency_config: Option<FrequencyConfig>,
    /// 规则生效开始时间
    pub valid_from: Option<DateTime<Utc>>,
    /// 规则生效结束时间
    pub valid_until: Option<DateTime<Utc>>,
}

impl CachedRule {
    /// 获取所有需要的徽章 ID
    pub fn get_required_badge_ids(&self) -> Vec<i64> {
        self.required_badges.iter().map(|r| r.badge_id).collect()
    }

    /// 检查规则是否在有效期内
    pub fn is_within_time_window(&self, now: DateTime<Utc>) -> bool {
        let after_start = self.valid_from.is_none_or(|t| now >= t);
        let before_end = self.valid_until.is_none_or(|t| now <= t);
        after_start && before_end
    }
}

/// 自动权益规则缓存
///
/// 按触发徽章 ID 索引规则，支持快速查询"获得某徽章后应触发哪些自动权益规则"
pub struct AutoBenefitRuleCache {
    pool: PgPool,
    /// 徽章 ID -> 以该徽章为触发条件的规则列表
    trigger_index: RwLock<HashMap<i64, Vec<CachedRule>>>,
    /// 上次刷新时间
    last_refresh: RwLock<Instant>,
    /// 缓存 TTL
    cache_ttl: Duration,
}

impl AutoBenefitRuleCache {
    /// 创建新的规则缓存
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            trigger_index: RwLock::new(HashMap::new()),
            // 初始时间设为过去，确保首次查询会触发刷新
            last_refresh: RwLock::new(Instant::now() - Duration::from_secs(3600)),
            cache_ttl: Duration::from_secs(300), // 5 分钟 TTL
        }
    }

    /// 使用自定义 TTL 创建规则缓存
    pub fn with_ttl(pool: PgPool, ttl_seconds: u64) -> Self {
        Self {
            pool,
            trigger_index: RwLock::new(HashMap::new()),
            last_refresh: RwLock::new(Instant::now() - Duration::from_secs(3600)),
            cache_ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// 获取以某徽章为触发条件的所有 auto_redeem=true 规则
    ///
    /// 如果缓存过期会自动刷新。刷新失败时使用过期缓存继续服务，保证可用性。
    pub async fn get_rules_by_trigger(&self, badge_id: i64) -> Vec<CachedRule> {
        // 检查并刷新过期缓存
        if self.needs_refresh().await {
            if let Err(e) = self.refresh().await {
                tracing::warn!(
                    error = %e,
                    "刷新自动权益规则缓存失败，使用过期缓存继续服务"
                );
            }
        }

        // 从缓存获取
        let index = self.trigger_index.read().await;
        index.get(&badge_id).cloned().unwrap_or_default()
    }

    /// 获取所有缓存的规则数量（用于监控）
    pub async fn get_total_rules_count(&self) -> usize {
        let index = self.trigger_index.read().await;
        // 去重计算（同一规则可能出现在多个徽章的索引中）
        let mut seen_rule_ids = std::collections::HashSet::new();
        for rules in index.values() {
            for rule in rules {
                seen_rule_ids.insert(rule.rule_id);
            }
        }
        seen_rule_ids.len()
    }

    /// 获取缓存的徽章索引数量（用于监控）
    pub async fn get_indexed_badges_count(&self) -> usize {
        self.trigger_index.read().await.len()
    }

    /// 检查缓存是否需要刷新
    async fn needs_refresh(&self) -> bool {
        let last = *self.last_refresh.read().await;
        last.elapsed() > self.cache_ttl
    }

    /// 刷新缓存
    ///
    /// 从数据库加载所有 auto_redeem=true 且 status='active' 的规则，
    /// 并按触发徽章重建索引。
    pub async fn refresh(&self) -> Result<(), sqlx::Error> {
        let start = Instant::now();

        // 从数据库加载规则
        let rules = self.load_auto_redeem_rules().await?;
        let rules_count = rules.len();

        // 构建触发索引：每个规则的所有 required_badges 都可以作为触发条件
        let mut index: HashMap<i64, Vec<CachedRule>> = HashMap::new();
        for rule in rules {
            for required in &rule.required_badges {
                index
                    .entry(required.badge_id)
                    .or_default()
                    .push(rule.clone());
            }
        }

        let indexed_badges = index.len();

        // 更新缓存
        *self.trigger_index.write().await = index;
        *self.last_refresh.write().await = Instant::now();

        tracing::info!(
            rules_count = rules_count,
            indexed_badges = indexed_badges,
            duration_ms = start.elapsed().as_millis() as u64,
            "自动权益规则缓存已刷新"
        );

        Ok(())
    }

    /// 从数据库加载 auto_redeem=true 的规则
    async fn load_auto_redeem_rules(&self) -> Result<Vec<CachedRule>, sqlx::Error> {
        // 查询所有 auto_redeem=true 且 status='active' 的规则
        let rows = sqlx::query_as::<_, AutoRedeemRuleRow>(
            r#"
            SELECT
                id,
                name,
                benefit_id,
                required_badges,
                frequency_type,
                frequency_limit,
                redeem_time_start,
                redeem_time_end
            FROM badge_redemption_rules
            WHERE auto_redeem = true AND status = 'active'
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        // 转换为缓存结构
        let rules = rows
            .into_iter()
            .filter_map(|row| {
                // 解析 required_badges JSON
                let required_badges = Self::parse_required_badges(&row.required_badges);
                if required_badges.is_empty() {
                    tracing::warn!(
                        rule_id = row.id,
                        "规则的 required_badges 为空或解析失败，跳过"
                    );
                    return None;
                }

                // 构建频率配置
                let frequency_config = Self::build_frequency_config(
                    row.frequency_type.as_deref(),
                    row.frequency_limit,
                );

                Some(CachedRule {
                    rule_id: row.id,
                    name: row.name,
                    required_badges,
                    benefit_id: row.benefit_id,
                    frequency_config,
                    valid_from: row.redeem_time_start,
                    valid_until: row.redeem_time_end,
                })
            })
            .collect();

        Ok(rules)
    }

    /// 解析 required_badges JSON
    ///
    /// 支持两种格式：
    /// - snake_case: [{"badge_id": 1, "quantity": 1}]（数据库原始格式）
    /// - camelCase: [{"badgeId": 1, "quantity": 1}]（API 格式）
    fn parse_required_badges(json: &serde_json::Value) -> Vec<RequiredBadge> {
        // 先尝试 camelCase 格式
        if let Ok(badges) = serde_json::from_value::<Vec<RequiredBadge>>(json.clone()) {
            return badges;
        }

        // 再尝试 snake_case 格式
        #[derive(serde::Deserialize)]
        struct SnakeCaseRequired {
            badge_id: i64,
            quantity: i32,
        }

        if let Ok(badges) = serde_json::from_value::<Vec<SnakeCaseRequired>>(json.clone()) {
            return badges
                .into_iter()
                .map(|b| RequiredBadge {
                    badge_id: b.badge_id,
                    quantity: b.quantity,
                })
                .collect();
        }

        tracing::warn!(json = ?json, "无法解析 required_badges JSON");
        vec![]
    }

    /// 从数据库频率字段构建 FrequencyConfig
    fn build_frequency_config(
        frequency_type: Option<&str>,
        frequency_limit: Option<i32>,
    ) -> Option<FrequencyConfig> {
        let limit = frequency_limit?;
        let freq_type = frequency_type?;

        let mut config = FrequencyConfig::default();
        match freq_type {
            "daily" => config.max_per_day = Some(limit),
            "weekly" => config.max_per_week = Some(limit),
            "monthly" => config.max_per_month = Some(limit),
            "account" | "total" => config.max_per_user = Some(limit),
            _ => {
                tracing::warn!(frequency_type = freq_type, "未知的频率限制类型");
                return None;
            }
        }

        Some(config)
    }

    /// 手动使缓存失效
    ///
    /// 规则变更时调用，强制下次查询时刷新缓存
    pub async fn invalidate(&self) {
        *self.last_refresh.write().await = Instant::now() - self.cache_ttl - Duration::from_secs(1);
        tracing::debug!("自动权益规则缓存已标记为失效");
    }

    /// 预热缓存
    ///
    /// 服务启动时调用，避免首次请求时的缓存 miss
    pub async fn warmup(&self) -> Result<(), sqlx::Error> {
        tracing::info!("开始预热自动权益规则缓存");
        self.refresh().await
    }
}

/// 数据库查询结果行
#[derive(Debug, sqlx::FromRow)]
struct AutoRedeemRuleRow {
    id: i64,
    name: String,
    benefit_id: i64,
    required_badges: serde_json::Value,
    frequency_type: Option<String>,
    frequency_limit: Option<i32>,
    redeem_time_start: Option<DateTime<Utc>>,
    redeem_time_end: Option<DateTime<Utc>>,
}

/// 创建共享的规则缓存实例
pub fn create_shared_cache(pool: PgPool) -> Arc<AutoBenefitRuleCache> {
    Arc::new(AutoBenefitRuleCache::new(pool))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_required_badges_camel_case() {
        let json = json!([
            {"badgeId": 1, "quantity": 2},
            {"badgeId": 3, "quantity": 1}
        ]);

        let badges = AutoBenefitRuleCache::parse_required_badges(&json);
        assert_eq!(badges.len(), 2);
        assert_eq!(badges[0].badge_id, 1);
        assert_eq!(badges[0].quantity, 2);
        assert_eq!(badges[1].badge_id, 3);
        assert_eq!(badges[1].quantity, 1);
    }

    #[test]
    fn test_parse_required_badges_snake_case() {
        let json = json!([
            {"badge_id": 5, "quantity": 1},
            {"badge_id": 6, "quantity": 3}
        ]);

        let badges = AutoBenefitRuleCache::parse_required_badges(&json);
        assert_eq!(badges.len(), 2);
        assert_eq!(badges[0].badge_id, 5);
        assert_eq!(badges[0].quantity, 1);
        assert_eq!(badges[1].badge_id, 6);
        assert_eq!(badges[1].quantity, 3);
    }

    #[test]
    fn test_parse_required_badges_invalid() {
        let json = json!({"invalid": "format"});
        let badges = AutoBenefitRuleCache::parse_required_badges(&json);
        assert!(badges.is_empty());
    }

    #[test]
    fn test_build_frequency_config_daily() {
        let config = AutoBenefitRuleCache::build_frequency_config(Some("daily"), Some(5));
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.max_per_day, Some(5));
        assert_eq!(config.max_per_user, None);
    }

    #[test]
    fn test_build_frequency_config_account() {
        let config = AutoBenefitRuleCache::build_frequency_config(Some("account"), Some(10));
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.max_per_user, Some(10));
        assert_eq!(config.max_per_day, None);
    }

    #[test]
    fn test_build_frequency_config_none() {
        let config = AutoBenefitRuleCache::build_frequency_config(None, Some(5));
        assert!(config.is_none());

        let config = AutoBenefitRuleCache::build_frequency_config(Some("daily"), None);
        assert!(config.is_none());
    }

    #[test]
    fn test_cached_rule_get_badge_ids() {
        let rule = CachedRule {
            rule_id: 1,
            name: "Test Rule".to_string(),
            required_badges: vec![
                RequiredBadge {
                    badge_id: 10,
                    quantity: 1,
                },
                RequiredBadge {
                    badge_id: 20,
                    quantity: 2,
                },
            ],
            benefit_id: 100,
            frequency_config: None,
            valid_from: None,
            valid_until: None,
        };

        let ids = rule.get_required_badge_ids();
        assert_eq!(ids, vec![10, 20]);
    }

    #[test]
    fn test_cached_rule_time_window() {
        let now = Utc::now();

        // 无时间限制
        let rule = CachedRule {
            rule_id: 1,
            name: "Test".to_string(),
            required_badges: vec![],
            benefit_id: 1,
            frequency_config: None,
            valid_from: None,
            valid_until: None,
        };
        assert!(rule.is_within_time_window(now));

        // 在有效期内
        let rule = CachedRule {
            rule_id: 1,
            name: "Test".to_string(),
            required_badges: vec![],
            benefit_id: 1,
            frequency_config: None,
            valid_from: Some(now - chrono::Duration::hours(1)),
            valid_until: Some(now + chrono::Duration::hours(1)),
        };
        assert!(rule.is_within_time_window(now));

        // 已过期
        let rule = CachedRule {
            rule_id: 1,
            name: "Test".to_string(),
            required_badges: vec![],
            benefit_id: 1,
            frequency_config: None,
            valid_from: None,
            valid_until: Some(now - chrono::Duration::hours(1)),
        };
        assert!(!rule.is_within_time_window(now));

        // 尚未生效
        let rule = CachedRule {
            rule_id: 1,
            name: "Test".to_string(),
            required_badges: vec![],
            benefit_id: 1,
            frequency_config: None,
            valid_from: Some(now + chrono::Duration::hours(1)),
            valid_until: None,
        };
        assert!(!rule.is_within_time_window(now));
    }
}
