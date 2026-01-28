//! 统计报表 API 处理器
//!
//! 提供统计总览、趋势分析、徽章排行和单徽章统计。
//! 所有查询基于 badges 和 badge_ledger 表聚合计算。

use axum::{
    extract::{Path, Query, State},
    Json,
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
    .bind(params.start_time)
    .bind(params.end_time)
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
    let badge_exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
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
            daily_trends: vec![
                TrendDataPoint {
                    date: "2025-01-01".to_string(),
                    issued_count: 10,
                    redeemed_count: 3,
                },
            ],
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"dailyTrends\""));
        assert!(json.contains("\"uniqueHolders\":150"));
    }
}
