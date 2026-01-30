//! 统计报表 API 处理器
//!
//! 提供统计总览、趋势分析、徽章排行和单徽章统计。
//! 所有查询基于 badges 和 badge_ledger 表聚合计算。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDate;
use tracing::instrument;

use crate::{
    dto::{
        ApiResponse, BadgeRankingDto, BadgeStatsDto, PaginationParams, StatsOverview,
        TimeRangeParams, TrendDataPoint,
    },
    error::AdminError,
    state::AppState,
};

/// 今日统计响应 DTO
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayStatsDto {
    /// 今日发放数量
    pub today_issued: i64,
    /// 今日兑换数量
    pub today_redeemed: i64,
    /// 今日新增持有用户数
    pub today_new_holders: i64,
    /// 发放环比变化（百分比）
    pub issued_change_rate: f64,
    /// 兑换环比变化（百分比）
    pub redeemed_change_rate: f64,
}

/// 徽章类型分布 DTO
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeDistributionDto {
    /// 类型名称
    pub badge_type: String,
    /// 数量
    pub count: i64,
    /// 百分比
    pub percentage: f64,
}

/// 统计总览
///
/// GET /api/admin/stats/overview
///
/// 返回全局统计指标：总徽章数、活跃徽章数、
/// 发放/兑换总量及当日数据。
#[instrument(skip(state))]
pub async fn get_overview(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<StatsOverview>>, AdminError> {
    let badge_counts: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*),
            COUNT(*) FILTER (WHERE status = 'active')
        FROM badges
        "#,
    )
    .fetch_one(&state.pool)
    .await?;

    let ledger_totals: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(quantity) FILTER (WHERE change_type = 'acquire'), 0),
            COALESCE(SUM(ABS(quantity)) FILTER (WHERE change_type = 'redeem_out'), 0)
        FROM badge_ledger
        "#,
    )
    .fetch_one(&state.pool)
    .await?;

    let today_totals: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(quantity) FILTER (WHERE change_type = 'acquire'), 0),
            COALESCE(SUM(ABS(quantity)) FILTER (WHERE change_type = 'redeem_out'), 0)
        FROM badge_ledger
        WHERE DATE(created_at) = CURRENT_DATE
        "#,
    )
    .fetch_one(&state.pool)
    .await?;

    let overview = StatsOverview {
        total_badges: badge_counts.0,
        active_badges: badge_counts.1,
        total_issued: ledger_totals.0,
        total_redeemed: ledger_totals.1,
        today_issued: today_totals.0,
        today_redeemed: today_totals.1,
    };

    Ok(Json(ApiResponse::success(overview)))
}

/// 趋势数据行
#[derive(sqlx::FromRow)]
struct TrendRow {
    date: NaiveDate,
    issued_count: i64,
    redeemed_count: i64,
}

/// 趋势数据查询
///
/// GET /api/admin/stats/trends
///
/// 按日期聚合指定时间范围内的发放和兑换数量，
/// 用于折线图等可视化展示。
#[instrument(skip(state))]
pub async fn get_trends(
    State(state): State<AppState>,
    Query(params): Query<TimeRangeParams>,
) -> Result<Json<ApiResponse<Vec<TrendDataPoint>>>, AdminError> {
    let rows = sqlx::query_as::<_, TrendRow>(
        r#"
        SELECT
            DATE(created_at) as date,
            COALESCE(SUM(quantity) FILTER (WHERE change_type = 'acquire'), 0) as issued_count,
            COALESCE(SUM(ABS(quantity)) FILTER (WHERE change_type = 'redeem_out'), 0) as redeemed_count
        FROM badge_ledger
        WHERE created_at >= $1 AND created_at <= $2
        GROUP BY DATE(created_at)
        ORDER BY date
        "#,
    )
    .bind(params.start_time())
    .bind(params.end_time())
    .fetch_all(&state.pool)
    .await?;

    let data_points: Vec<TrendDataPoint> = rows
        .into_iter()
        .map(|row| TrendDataPoint {
            date: row.date.format("%Y-%m-%d").to_string(),
            issued_count: row.issued_count,
            redeemed_count: row.redeemed_count,
        })
        .collect();

    Ok(Json(ApiResponse::success(data_points)))
}

/// 排行榜行
#[derive(sqlx::FromRow)]
struct RankingRow {
    badge_id: i64,
    badge_name: String,
    badge_type: String,
    total_issued: i64,
    total_redeemed: i64,
    active_holders: i64,
}

/// 徽章排行榜
///
/// GET /api/admin/stats/ranking
///
/// 按发放量降序排列所有徽章，同时附带兑换量和活跃持有人数。
#[instrument(skip(state))]
pub async fn get_ranking(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<Vec<BadgeRankingDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let rows = sqlx::query_as::<_, RankingRow>(
        r#"
        SELECT
            b.id as badge_id,
            b.name as badge_name,
            b.badge_type::text as badge_type,
            COALESCE(SUM(bl.quantity) FILTER (WHERE bl.change_type = 'acquire'), 0) as total_issued,
            COALESCE(SUM(ABS(bl.quantity)) FILTER (WHERE bl.change_type = 'redeem_out'), 0) as total_redeemed,
            COUNT(DISTINCT bl.user_id) FILTER (WHERE bl.change_type = 'acquire') as active_holders
        FROM badges b
        LEFT JOIN badge_ledger bl ON b.id = bl.badge_id
        GROUP BY b.id, b.name, b.badge_type
        ORDER BY total_issued DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let ranking: Vec<BadgeRankingDto> = rows
        .into_iter()
        .map(|row| BadgeRankingDto {
            badge_id: row.badge_id,
            badge_name: row.badge_name,
            badge_type: row.badge_type,
            total_issued: row.total_issued,
            total_redeemed: row.total_redeemed,
            active_holders: row.active_holders,
        })
        .collect();

    Ok(Json(ApiResponse::success(ranking)))
}

/// 单徽章统计行
#[derive(sqlx::FromRow)]
struct BadgeStatsRow {
    badge_id: i64,
    badge_name: String,
    total_issued: i64,
    total_redeemed: i64,
    unique_holders: i64,
    today_issued: i64,
    today_redeemed: i64,
}

/// 单徽章统计详情
///
/// GET /api/admin/stats/badges/:id
///
/// 返回指定徽章的发放/兑换统计及近 30 天趋势数据。
#[instrument(skip(state))]
pub async fn get_badge_stats(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BadgeStatsDto>>, AdminError> {
    // 验证徽章存在
    let badge_exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if !badge_exists.0 {
        return Err(AdminError::BadgeNotFound(id));
    }

    let stats = sqlx::query_as::<_, BadgeStatsRow>(
        r#"
        SELECT
            b.id as badge_id,
            b.name as badge_name,
            COALESCE(SUM(bl.quantity) FILTER (WHERE bl.change_type = 'acquire'), 0) as total_issued,
            COALESCE(SUM(ABS(bl.quantity)) FILTER (WHERE bl.change_type = 'redeem_out'), 0) as total_redeemed,
            COUNT(DISTINCT bl.user_id) FILTER (WHERE bl.change_type = 'acquire') as unique_holders,
            COALESCE(SUM(bl.quantity) FILTER (WHERE bl.change_type = 'acquire' AND DATE(bl.created_at) = CURRENT_DATE), 0) as today_issued,
            COALESCE(SUM(ABS(bl.quantity)) FILTER (WHERE bl.change_type = 'redeem_out' AND DATE(bl.created_at) = CURRENT_DATE), 0) as today_redeemed
        FROM badges b
        LEFT JOIN badge_ledger bl ON b.id = bl.badge_id
        WHERE b.id = $1
        GROUP BY b.id, b.name
        "#,
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    // 近 30 天趋势
    let trend_rows = sqlx::query_as::<_, TrendRow>(
        r#"
        SELECT
            DATE(created_at) as date,
            COALESCE(SUM(quantity) FILTER (WHERE change_type = 'acquire'), 0) as issued_count,
            COALESCE(SUM(ABS(quantity)) FILTER (WHERE change_type = 'redeem_out'), 0) as redeemed_count
        FROM badge_ledger
        WHERE badge_id = $1
          AND created_at >= CURRENT_DATE - INTERVAL '30 days'
        GROUP BY DATE(created_at)
        ORDER BY date
        "#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    let daily_trends: Vec<TrendDataPoint> = trend_rows
        .into_iter()
        .map(|row| TrendDataPoint {
            date: row.date.format("%Y-%m-%d").to_string(),
            issued_count: row.issued_count,
            redeemed_count: row.redeemed_count,
        })
        .collect();

    let dto = BadgeStatsDto {
        badge_id: stats.badge_id,
        badge_name: stats.badge_name,
        total_issued: stats.total_issued,
        total_redeemed: stats.total_redeemed,
        unique_holders: stats.unique_holders,
        today_issued: stats.today_issued,
        today_redeemed: stats.today_redeemed,
        daily_trends,
    };

    Ok(Json(ApiResponse::success(dto)))
}

/// 今日统计数据
///
/// GET /api/admin/stats/today
///
/// 返回当日运营数据及环比变化
#[instrument(skip(state))]
pub async fn get_today_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<TodayStatsDto>>, AdminError> {
    // 今日数据
    let today_stats: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(quantity) FILTER (WHERE change_type = 'acquire'), 0),
            COALESCE(SUM(ABS(quantity)) FILTER (WHERE change_type = 'redeem_out'), 0)
        FROM badge_ledger
        WHERE DATE(created_at) = CURRENT_DATE
        "#,
    )
    .fetch_one(&state.pool)
    .await?;

    // 昨日数据（用于计算环比）
    let yesterday_stats: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(quantity) FILTER (WHERE change_type = 'acquire'), 0),
            COALESCE(SUM(ABS(quantity)) FILTER (WHERE change_type = 'redeem_out'), 0)
        FROM badge_ledger
        WHERE DATE(created_at) = CURRENT_DATE - INTERVAL '1 day'
        "#,
    )
    .fetch_one(&state.pool)
    .await?;

    // 今日新增持有用户数
    let today_new_holders: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT user_id)
        FROM user_badges
        WHERE DATE(first_acquired_at) = CURRENT_DATE
        "#,
    )
    .fetch_one(&state.pool)
    .await?;

    // 计算环比变化率
    let issued_change_rate = if yesterday_stats.0 > 0 {
        ((today_stats.0 - yesterday_stats.0) as f64 / yesterday_stats.0 as f64) * 100.0
    } else if today_stats.0 > 0 {
        100.0
    } else {
        0.0
    };

    let redeemed_change_rate = if yesterday_stats.1 > 0 {
        ((today_stats.1 - yesterday_stats.1) as f64 / yesterday_stats.1 as f64) * 100.0
    } else if today_stats.1 > 0 {
        100.0
    } else {
        0.0
    };

    let dto = TodayStatsDto {
        today_issued: today_stats.0,
        today_redeemed: today_stats.1,
        today_new_holders: today_new_holders.0,
        issued_change_rate,
        redeemed_change_rate,
    };

    Ok(Json(ApiResponse::success(dto)))
}

/// 徽章类型分布
///
/// GET /api/admin/stats/distribution/types
///
/// 用于饼图展示各类型徽章的发放占比
#[instrument(skip(state))]
pub async fn get_type_distribution(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<TypeDistributionDto>>>, AdminError> {
    // 按类型统计发放数量
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT
            b.badge_type::text as badge_type,
            COALESCE(SUM(bl.quantity) FILTER (WHERE bl.change_type = 'acquire'), 0) as count
        FROM badges b
        LEFT JOIN badge_ledger bl ON b.id = bl.badge_id
        GROUP BY b.badge_type
        ORDER BY count DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    // 计算总数以得出百分比
    let total: i64 = rows.iter().map(|(_, c)| c).sum();

    let distributions: Vec<TypeDistributionDto> = rows
        .into_iter()
        .map(|(badge_type, count)| TypeDistributionDto {
            badge_type,
            count,
            percentage: if total > 0 {
                (count as f64 / total as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect();

    Ok(Json(ApiResponse::success(distributions)))
}

/// 用户活跃度趋势数据
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityTrendPoint {
    pub date: String,
    pub value: i64,
}

/// 活跃度行
#[derive(sqlx::FromRow)]
struct ActivityRow {
    date: NaiveDate,
    active_users: i64,
}

/// 用户活跃度趋势
///
/// GET /api/admin/stats/trend/activity
///
/// 按日期聚合指定时间范围内的活跃用户数（有徽章变动的用户）
#[instrument(skip(state))]
pub async fn get_activity_trend(
    State(state): State<AppState>,
    Query(params): Query<TimeRangeParams>,
) -> Result<Json<ApiResponse<Vec<ActivityTrendPoint>>>, AdminError> {
    let rows = sqlx::query_as::<_, ActivityRow>(
        r#"
        SELECT
            DATE(created_at) as date,
            COUNT(DISTINCT user_id) as active_users
        FROM badge_ledger
        WHERE created_at >= $1 AND created_at <= $2
        GROUP BY DATE(created_at)
        ORDER BY date
        "#,
    )
    .bind(params.start_time())
    .bind(params.end_time())
    .fetch_all(&state.pool)
    .await?;

    let data_points: Vec<ActivityTrendPoint> = rows
        .into_iter()
        .map(|row| ActivityTrendPoint {
            date: row.date.format("%Y-%m-%d").to_string(),
            value: row.active_users,
        })
        .collect();

    Ok(Json(ApiResponse::success(data_points)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_row_to_data_point() {
        let row = TrendRow {
            date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            issued_count: 100,
            redeemed_count: 30,
        };

        let point = TrendDataPoint {
            date: row.date.format("%Y-%m-%d").to_string(),
            issued_count: row.issued_count,
            redeemed_count: row.redeemed_count,
        };

        assert_eq!(point.date, "2025-01-15");
        assert_eq!(point.issued_count, 100);
        assert_eq!(point.redeemed_count, 30);
    }

    #[test]
    fn test_stats_overview_default() {
        let overview = StatsOverview::default();
        assert_eq!(overview.total_badges, 0);
        assert_eq!(overview.active_badges, 0);
        assert_eq!(overview.total_issued, 0);
        assert_eq!(overview.total_redeemed, 0);
        assert_eq!(overview.today_issued, 0);
        assert_eq!(overview.today_redeemed, 0);
    }

    #[test]
    fn test_badge_ranking_dto_serialization() {
        let dto = BadgeRankingDto {
            badge_id: 1,
            badge_name: "VIP 徽章".to_string(),
            badge_type: "normal".to_string(),
            total_issued: 500,
            total_redeemed: 100,
            active_holders: 350,
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"badgeId\":1"));
        assert!(json.contains("\"totalIssued\":500"));
        assert!(json.contains("\"activeHolders\":350"));
    }

    #[test]
    fn test_badge_stats_dto_serialization() {
        let dto = BadgeStatsDto {
            badge_id: 1,
            badge_name: "测试徽章".to_string(),
            total_issued: 200,
            total_redeemed: 50,
            unique_holders: 150,
            today_issued: 10,
            today_redeemed: 3,
            daily_trends: vec![TrendDataPoint {
                date: "2025-01-01".to_string(),
                issued_count: 10,
                redeemed_count: 3,
            }],
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"dailyTrends\""));
        assert!(json.contains("\"uniqueHolders\":150"));
    }
}
