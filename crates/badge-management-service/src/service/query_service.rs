//! 徽章查询服务
//!
//! 提供徽章相关的只读查询功能，采用缓存优先策略以提升性能。
//!
//! ## 缓存策略
//!
//! - 用户徽章列表: TTL 5 分钟
//! - 徽章详情: TTL 10 分钟
//! - 徽章墙: TTL 5 分钟
//! - 分类列表: TTL 30 分钟

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{info, instrument, warn};

use badge_shared::cache::Cache;

use crate::error::{BadgeError, Result};
use crate::models::{Badge, BadgeAssets, UserBadgeStatus};
use crate::repository::{
    BadgeLedgerRepositoryTrait, BadgeRepositoryTrait, RedemptionRepositoryTrait,
    UserBadgeRepositoryTrait,
};
use crate::service::dto::{
    BadgeDetailDto, BadgeSummaryDto, BadgeWallCategoryDto, BadgeWallDto, BenefitSummaryDto,
    CategoryDto, CategoryStatsDto, SeriesDetailDto, UserBadgeDto, UserBadgeStatsDto,
};

/// 缓存 TTL 常量（秒）
mod cache_ttl {
    pub const USER_BADGES: u64 = 300; // 5 min
    pub const BADGE_DETAIL: u64 = 600; // 10 min
    pub const BADGE_WALL: u64 = 300; // 5 min
    pub const CATEGORIES: u64 = 1800; // 30 min
}

/// 缓存键生成
mod cache_keys {
    pub fn user_badges(user_id: &str) -> String {
        format!("user:badge:{}", user_id)
    }

    pub fn badge_detail(badge_id: i64) -> String {
        format!("badge:detail:{}", badge_id)
    }

    pub fn badge_wall(user_id: &str) -> String {
        format!("user:badge:wall:{}", user_id)
    }

    pub fn categories() -> String {
        "badge:categories".to_string()
    }
}

/// 徽章查询服务
///
/// 聚合多个仓储提供完整的徽章查询能力，内置缓存以满足高并发场景的性能需求
pub struct BadgeQueryService<BR, UBR, RR, LR>
where
    BR: BadgeRepositoryTrait,
    UBR: UserBadgeRepositoryTrait,
    RR: RedemptionRepositoryTrait,
    LR: BadgeLedgerRepositoryTrait,
{
    badge_repo: Arc<BR>,
    user_badge_repo: Arc<UBR>,
    redemption_repo: Arc<RR>,
    #[allow(dead_code)]
    ledger_repo: Arc<LR>,
    cache: Arc<Cache>,
}

impl<BR, UBR, RR, LR> BadgeQueryService<BR, UBR, RR, LR>
where
    BR: BadgeRepositoryTrait,
    UBR: UserBadgeRepositoryTrait,
    RR: RedemptionRepositoryTrait,
    LR: BadgeLedgerRepositoryTrait,
{
    pub fn new(
        badge_repo: Arc<BR>,
        user_badge_repo: Arc<UBR>,
        redemption_repo: Arc<RR>,
        ledger_repo: Arc<LR>,
        cache: Arc<Cache>,
    ) -> Self {
        Self {
            badge_repo,
            user_badge_repo,
            redemption_repo,
            ledger_repo,
            cache,
        }
    }

    /// 带缓存的数据获取辅助方法
    ///
    /// 在缓存层和业务层之间转换错误类型，避免直接依赖 badge_shared::error::Result
    async fn get_cached_or_fetch<T, F, Fut>(&self, key: &str, ttl: Duration, fetch: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        // 尝试从缓存获取
        match self.cache.get::<T>(key).await {
            Ok(Some(cached)) => return Ok(cached),
            Ok(None) => {}
            Err(e) => {
                // 缓存读取失败时记录警告，继续从数据源获取
                warn!(key = %key, error = %e, "Cache get failed, falling back to database");
            }
        }

        // 从数据源获取
        let data = fetch().await?;

        // 异步写入缓存，写入失败不影响主流程
        if let Err(e) = self.cache.set(key, &data, ttl).await {
            warn!(key = %key, error = %e, "Cache set failed");
        }

        Ok(data)
    }

    /// 获取用户徽章列表
    ///
    /// 返回用户持有的所有徽章，聚合徽章定义信息。
    /// 缓存键: user:badge:{user_id}, TTL: 5min
    #[instrument(skip(self), fields(user_id = %user_id))]
    pub async fn get_user_badges(&self, user_id: &str) -> Result<Vec<UserBadgeDto>> {
        let cache_key = cache_keys::user_badges(user_id);
        let user_id_owned = user_id.to_string();

        self.get_cached_or_fetch(
            &cache_key,
            Duration::from_secs(cache_ttl::USER_BADGES),
            || async { self.fetch_user_badges(&user_id_owned).await },
        )
        .await
    }

    /// 从数据库获取用户徽章列表
    async fn fetch_user_badges(&self, user_id: &str) -> Result<Vec<UserBadgeDto>> {
        let user_badges = self.user_badge_repo.list_user_badges(user_id).await?;

        if user_badges.is_empty() {
            return Ok(vec![]);
        }

        // 批量获取徽章定义，避免 N+1 查询
        let badge_ids: Vec<i64> = user_badges.iter().map(|ub| ub.badge_id).collect();
        let badges = self.badge_repo.get_badges_by_ids(&badge_ids).await?;
        let badge_map: HashMap<i64, Badge> = badges.into_iter().map(|b| (b.id, b)).collect();

        let mut result = Vec::with_capacity(user_badges.len());
        for ub in user_badges {
            if let Some(badge) = badge_map.get(&ub.badge_id) {
                let assets = badge.parse_assets().unwrap_or_else(|e| {
                    warn!(badge_id = badge.id, error = %e, "Failed to parse badge assets");
                    BadgeAssets {
                        icon_url: String::new(),
                        image_url: None,
                        animation_url: None,
                        disabled_icon_url: None,
                    }
                });

                result.push(UserBadgeDto {
                    badge_id: ub.badge_id,
                    badge_name: badge.name.clone(),
                    badge_type: badge.badge_type,
                    quantity: ub.quantity,
                    status: ub.status,
                    acquired_at: ub.acquired_at,
                    expires_at: ub.expires_at,
                    assets,
                });
            }
        }

        info!(user_id = %user_id, count = result.len(), "Fetched user badges from database");
        Ok(result)
    }

    /// 获取徽章详情
    ///
    /// 返回徽章的完整信息，包括所属系列、分类和可兑换权益。
    /// 缓存键: badge:detail:{badge_id}, TTL: 10min
    #[instrument(skip(self), fields(badge_id = %badge_id))]
    pub async fn get_badge_detail(&self, badge_id: i64) -> Result<BadgeDetailDto> {
        let cache_key = cache_keys::badge_detail(badge_id);

        self.get_cached_or_fetch(
            &cache_key,
            Duration::from_secs(cache_ttl::BADGE_DETAIL),
            || async { self.fetch_badge_detail(badge_id).await },
        )
        .await
    }

    /// 从数据库获取徽章详情
    async fn fetch_badge_detail(&self, badge_id: i64) -> Result<BadgeDetailDto> {
        let badge = self
            .badge_repo
            .get_badge(badge_id)
            .await?
            .ok_or(BadgeError::BadgeNotFound(badge_id))?;

        // 获取系列信息
        let series = self
            .badge_repo
            .get_series(badge.series_id)
            .await?
            .ok_or(BadgeError::SeriesNotFound(badge.series_id))?;

        // 获取分类信息
        let category = self
            .badge_repo
            .get_category(series.category_id)
            .await?
            .ok_or(BadgeError::CategoryNotFound(series.category_id))?;

        // 获取可兑换权益
        let rules = self.redemption_repo.list_rules_by_badge(badge_id).await?;
        let now = Utc::now();
        let active_rules: Vec<_> = rules.into_iter().filter(|r| r.is_active(now)).collect();

        let mut redeemable_benefits = Vec::new();
        for rule in active_rules {
            if let Ok(Some(benefit)) = self.redemption_repo.get_benefit(rule.benefit_id).await {
                // 解析该规则需要当前徽章的数量
                let required_qty = rule
                    .parse_required_badges()
                    .ok()
                    .and_then(|badges| {
                        badges
                            .iter()
                            .find(|b| b.badge_id == badge_id)
                            .map(|b| b.quantity)
                    })
                    .unwrap_or(1);

                redeemable_benefits.push(BenefitSummaryDto {
                    benefit_id: benefit.id,
                    benefit_type: benefit.benefit_type,
                    name: benefit.name,
                    description: benefit.description,
                    icon_url: benefit.icon_url,
                    required_quantity: required_qty,
                });
            }
        }

        let assets = badge.parse_assets()?;
        let validity_config = badge.parse_validity_config().unwrap_or_default();

        info!(badge_id = %badge_id, "Fetched badge detail from database");

        Ok(BadgeDetailDto {
            id: badge.id,
            name: badge.name,
            description: badge.description.unwrap_or_default(),
            badge_type: badge.badge_type,
            series_id: series.id,
            series_name: series.name,
            category_id: category.id,
            category_name: category.name,
            assets,
            obtain_description: badge.obtain_description.unwrap_or_default(),
            validity_config,
            max_supply: badge.max_supply.map(|s| s as i32),
            issued_count: badge.issued_count as i32,
            redeemable_benefits,
        })
    }

    /// 获取用户徽章墙
    ///
    /// 按分类组织用户的所有徽章，便于前端展示徽章墙视图。
    /// 缓存键: user:badge:wall:{user_id}, TTL: 5min
    #[instrument(skip(self), fields(user_id = %user_id))]
    pub async fn get_badge_wall(&self, user_id: &str) -> Result<BadgeWallDto> {
        let cache_key = cache_keys::badge_wall(user_id);
        let user_id_owned = user_id.to_string();

        self.get_cached_or_fetch(
            &cache_key,
            Duration::from_secs(cache_ttl::BADGE_WALL),
            || async { self.fetch_badge_wall(&user_id_owned).await },
        )
        .await
    }

    /// 从数据库构建徽章墙
    async fn fetch_badge_wall(&self, user_id: &str) -> Result<BadgeWallDto> {
        // 获取用户徽章（已包含徽章定义信息）
        let user_badges = self.fetch_user_badges(user_id).await?;

        if user_badges.is_empty() {
            return Ok(BadgeWallDto {
                total_count: 0,
                categories: vec![],
            });
        }

        // 获取所有徽章定义以查询其系列信息
        let badge_ids: Vec<i64> = user_badges.iter().map(|ub| ub.badge_id).collect();
        let badges = self.badge_repo.get_badges_by_ids(&badge_ids).await?;

        // 批量获取所有涉及的系列，避免 N+1 查询
        let series_ids: Vec<i64> = badges
            .iter()
            .map(|b| b.series_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let all_series = self.badge_repo.get_series_by_ids(&series_ids).await?;

        // 构建系列 -> 分类映射
        let series_category_map: HashMap<i64, i64> = all_series
            .iter()
            .map(|s| (s.id, s.category_id))
            .collect();

        // 批量获取所有涉及的分类，避免 N+1 查询
        let category_ids: Vec<i64> = all_series
            .iter()
            .map(|s| s.category_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let categories = self.badge_repo.get_categories_by_ids(&category_ids).await?;
        let category_names: HashMap<i64, String> = categories
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect();

        // 建立 badge_id -> category_id 映射
        let badge_category_map: HashMap<i64, i64> = badges
            .iter()
            .filter_map(|b| {
                series_category_map
                    .get(&b.series_id)
                    .map(|&cat_id| (b.id, cat_id))
            })
            .collect();

        // 按分类分组徽章
        let mut category_badges: HashMap<i64, Vec<UserBadgeDto>> = HashMap::new();
        for badge in user_badges.iter() {
            if let Some(&cat_id) = badge_category_map.get(&badge.badge_id) {
                category_badges
                    .entry(cat_id)
                    .or_default()
                    .push(badge.clone());
            }
        }

        // 构建响应
        let categories: Vec<BadgeWallCategoryDto> = category_badges
            .into_iter()
            .filter_map(|(cat_id, badges)| {
                category_names.get(&cat_id).map(|name| BadgeWallCategoryDto {
                    category_id: cat_id,
                    category_name: name.clone(),
                    badges,
                })
            })
            .collect();

        info!(user_id = %user_id, category_count = categories.len(), "Built badge wall from database");

        Ok(BadgeWallDto {
            total_count: user_badges.len() as i32,
            categories,
        })
    }

    /// 获取徽章分类列表
    ///
    /// 返回所有启用的分类及各分类下的徽章数量。
    /// 缓存键: badge:categories, TTL: 30min
    #[instrument(skip(self))]
    pub async fn get_categories(&self) -> Result<Vec<CategoryDto>> {
        let cache_key = cache_keys::categories();

        self.get_cached_or_fetch(
            &cache_key,
            Duration::from_secs(cache_ttl::CATEGORIES),
            || async { self.fetch_categories().await },
        )
        .await
    }

    /// 从数据库获取分类列表
    async fn fetch_categories(&self) -> Result<Vec<CategoryDto>> {
        let categories = self.badge_repo.list_categories().await?;
        let all_badges = self.badge_repo.list_active_badges().await?;

        // 批量获取所有涉及的系列，避免 N+1 查询
        let series_ids: Vec<i64> = all_badges
            .iter()
            .map(|b| b.series_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let all_series = self.badge_repo.get_series_by_ids(&series_ids).await?;
        let series_map: HashMap<i64, i64> = all_series
            .iter()
            .map(|s| (s.id, s.category_id))
            .collect();

        // 统计各分类下的徽章数量
        let mut badge_counts: HashMap<i64, i32> = HashMap::new();
        for badge in &all_badges {
            if let Some(&category_id) = series_map.get(&badge.series_id) {
                *badge_counts.entry(category_id).or_default() += 1;
            }
        }

        let result: Vec<CategoryDto> = categories
            .into_iter()
            .map(|cat| CategoryDto {
                id: cat.id,
                name: cat.name,
                icon_url: cat.icon_url,
                badge_count: badge_counts.get(&cat.id).copied().unwrap_or(0),
            })
            .collect();

        info!(count = result.len(), "Fetched categories from database");
        Ok(result)
    }

    /// 获取系列详情
    ///
    /// 返回系列的完整信息，包含系列内所有徽章。
    #[instrument(skip(self), fields(series_id = %series_id))]
    pub async fn get_series_detail(&self, series_id: i64) -> Result<SeriesDetailDto> {
        let series = self
            .badge_repo
            .get_series(series_id)
            .await?
            .ok_or(BadgeError::SeriesNotFound(series_id))?;

        let category = self
            .badge_repo
            .get_category(series.category_id)
            .await?
            .ok_or(BadgeError::CategoryNotFound(series.category_id))?;

        let badges = self.badge_repo.list_badges_by_series(series_id).await?;

        let badge_summaries: Vec<BadgeSummaryDto> = badges
            .into_iter()
            .filter_map(|b| {
                b.parse_assets().ok().map(|assets| BadgeSummaryDto {
                    id: b.id,
                    name: b.name,
                    badge_type: b.badge_type,
                    assets,
                    max_supply: b.max_supply.map(|s| s as i32),
                    issued_count: b.issued_count as i32,
                })
            })
            .collect();

        info!(series_id = %series_id, badge_count = badge_summaries.len(), "Fetched series detail");

        Ok(SeriesDetailDto {
            id: series.id,
            category_id: category.id,
            category_name: category.name,
            name: series.name,
            description: series.description,
            cover_url: series.cover_url,
            start_time: series.start_time,
            end_time: series.end_time,
            badges: badge_summaries,
        })
    }

    /// 获取用户徽章统计
    ///
    /// 返回用户徽章的汇总统计，包括总数、按状态统计和按分类统计。
    #[instrument(skip(self), fields(user_id = %user_id))]
    pub async fn get_user_badge_stats(&self, user_id: &str) -> Result<UserBadgeStatsDto> {
        let user_badges = self.user_badge_repo.list_user_badges(user_id).await?;

        if user_badges.is_empty() {
            return Ok(UserBadgeStatsDto {
                total_badges: 0,
                active_badges: 0,
                expired_badges: 0,
                redeemed_badges: 0,
                by_category: vec![],
            });
        }

        let now = Utc::now();
        let mut active_count = 0i32;
        let mut expired_count = 0i32;
        let mut redeemed_count = 0i32;

        for ub in &user_badges {
            match ub.status {
                UserBadgeStatus::Active => {
                    if ub.is_expired(now) {
                        expired_count += ub.quantity;
                    } else {
                        active_count += ub.quantity;
                    }
                }
                UserBadgeStatus::Expired => expired_count += ub.quantity,
                UserBadgeStatus::Redeemed => redeemed_count += ub.quantity,
                UserBadgeStatus::Revoked => {}
            }
        }

        // 按分类统计
        let badge_ids: Vec<i64> = user_badges.iter().map(|ub| ub.badge_id).collect();
        let badges = self.badge_repo.get_badges_by_ids(&badge_ids).await?;

        // badge_id -> quantity 映射
        let qty_map: HashMap<i64, i32> = user_badges
            .iter()
            .map(|ub| (ub.badge_id, ub.quantity))
            .collect();

        // 批量获取所有涉及的系列，避免 N+1 查询
        let series_ids: Vec<i64> = badges
            .iter()
            .map(|b| b.series_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let all_series = self.badge_repo.get_series_by_ids(&series_ids).await?;
        let series_map: HashMap<i64, i64> = all_series
            .iter()
            .map(|s| (s.id, s.category_id))
            .collect();

        // 统计各分类的徽章数量
        let mut category_counts: HashMap<i64, i32> = HashMap::new();
        for badge in &badges {
            if let Some(&category_id) = series_map.get(&badge.series_id) {
                let qty = qty_map.get(&badge.id).copied().unwrap_or(0);
                *category_counts.entry(category_id).or_default() += qty;
            }
        }

        // 批量获取分类信息，避免 N+1 查询
        let category_ids: Vec<i64> = category_counts.keys().copied().collect();
        let categories = self.badge_repo.get_categories_by_ids(&category_ids).await?;
        let category_map: HashMap<i64, String> = categories
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect();

        // 构建分类统计结果
        let by_category: Vec<CategoryStatsDto> = category_counts
            .into_iter()
            .filter_map(|(cat_id, count)| {
                category_map.get(&cat_id).map(|name| CategoryStatsDto {
                    category_id: cat_id,
                    category_name: name.clone(),
                    count,
                })
            })
            .collect();

        let total = user_badges.iter().map(|ub| ub.quantity).sum();

        info!(
            user_id = %user_id,
            total = total,
            active = active_count,
            expired = expired_count,
            redeemed = redeemed_count,
            "Calculated user badge stats"
        );

        Ok(UserBadgeStatsDto {
            total_badges: total,
            active_badges: active_count,
            expired_badges: expired_count,
            redeemed_badges: redeemed_count,
            by_category,
        })
    }

    /// 使指定用户的徽章缓存失效
    ///
    /// 当用户徽章发生变更时调用，确保下次查询获取最新数据
    pub async fn invalidate_user_cache(&self, user_id: &str) -> Result<()> {
        let keys = [
            cache_keys::user_badges(user_id),
            cache_keys::badge_wall(user_id),
        ];

        for key in keys {
            if let Err(e) = self.cache.delete(&key).await {
                warn!(key = %key, error = %e, "Failed to invalidate cache");
            }
        }

        Ok(())
    }

    /// 使徽章详情缓存失效
    ///
    /// 缓存删除失败仅记录警告，不传播错误，保持与 invalidate_user_cache 一致的行为
    pub async fn invalidate_badge_cache(&self, badge_id: i64) {
        let key = cache_keys::badge_detail(badge_id);
        if let Err(e) = self.cache.delete(&key).await {
            warn!(key = %key, error = %e, "Failed to invalidate badge cache");
        }
    }

    /// 使分类缓存失效
    ///
    /// 缓存删除失败仅记录警告，不传播错误，保持与 invalidate_user_cache 一致的行为
    pub async fn invalidate_categories_cache(&self) {
        let key = cache_keys::categories();
        if let Err(e) = self.cache.delete(&key).await {
            warn!(key = %key, error = %e, "Failed to invalidate categories cache");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Badge, BadgeCategory, BadgeSeries, BadgeStatus, CategoryStatus};
    use chrono::Utc;
    use serde_json::json;

    fn create_test_badge(id: i64, series_id: i64) -> Badge {
        Badge {
            id,
            series_id,
            badge_type: crate::models::BadgeType::Normal,
            name: format!("Badge {}", id),
            description: Some("Test badge".to_string()),
            obtain_description: Some("Complete the task".to_string()),
            sort_order: 0,
            status: BadgeStatus::Active,
            assets: json!({
                "iconUrl": "https://example.com/icon.png",
                "imageUrl": "https://example.com/image.png"
            }),
            validity_config: json!({"validityType": "PERMANENT"}),
            max_supply: Some(1000),
            issued_count: 100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_user_badge(user_id: &str, badge_id: i64) -> crate::models::UserBadge {
        crate::models::UserBadge {
            id: badge_id * 100,
            user_id: user_id.to_string(),
            badge_id,
            status: UserBadgeStatus::Active,
            quantity: 1,
            acquired_at: Utc::now(),
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_series(id: i64, category_id: i64) -> BadgeSeries {
        BadgeSeries {
            id,
            category_id,
            name: format!("Series {}", id),
            description: Some("Test series".to_string()),
            cover_url: None,
            sort_order: 0,
            status: CategoryStatus::Active,
            start_time: None,
            end_time: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_category(id: i64) -> BadgeCategory {
        BadgeCategory {
            id,
            name: format!("Category {}", id),
            icon_url: None,
            sort_order: 0,
            status: CategoryStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(cache_keys::user_badges("user-123"), "user:badge:user-123");
        assert_eq!(cache_keys::badge_detail(1), "badge:detail:1");
        assert_eq!(
            cache_keys::badge_wall("user-123"),
            "user:badge:wall:user-123"
        );
        assert_eq!(cache_keys::categories(), "badge:categories");
    }

    #[test]
    fn test_cache_ttl_values() {
        assert_eq!(cache_ttl::USER_BADGES, 300);
        assert_eq!(cache_ttl::BADGE_DETAIL, 600);
        assert_eq!(cache_ttl::BADGE_WALL, 300);
        assert_eq!(cache_ttl::CATEGORIES, 1800);
    }

    #[test]
    fn test_user_badge_stats_calculation() {
        let now = Utc::now();

        let mut active_badge = create_test_user_badge("user-1", 1);
        active_badge.status = UserBadgeStatus::Active;
        active_badge.quantity = 3;
        assert!(!active_badge.is_expired(now));

        let mut expired_badge = create_test_user_badge("user-1", 2);
        expired_badge.status = UserBadgeStatus::Expired;
        expired_badge.quantity = 2;

        let mut redeemed_badge = create_test_user_badge("user-1", 3);
        redeemed_badge.status = UserBadgeStatus::Redeemed;
        redeemed_badge.quantity = 1;

        // 计算预期统计
        let badges = vec![active_badge, expired_badge, redeemed_badge];
        let total: i32 = badges.iter().map(|b| b.quantity).sum();
        assert_eq!(total, 6);
    }

    #[test]
    fn test_badge_assets_parsing() {
        let badge = create_test_badge(1, 1);
        let assets = badge.parse_assets().unwrap();
        assert_eq!(assets.icon_url, "https://example.com/icon.png");
        assert_eq!(
            assets.image_url,
            Some("https://example.com/image.png".to_string())
        );
    }

    #[test]
    fn test_series_creation() {
        let series = create_test_series(1, 1);
        assert_eq!(series.id, 1);
        assert_eq!(series.category_id, 1);
        assert_eq!(series.name, "Series 1");
    }

    #[test]
    fn test_category_creation() {
        let category = create_test_category(1);
        assert_eq!(category.id, 1);
        assert_eq!(category.name, "Category 1");
    }
}
