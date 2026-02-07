//! 自动权益管理 API 处理器
//!
//! 提供自动权益发放记录和评估日志的查询接口。
//! 自动权益是指用户获得徽章后，系统自动评估并发放匹配的权益。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    dto::{ApiResponse, PageResponse, PaginationParams},
    error::AdminError,
    state::AppState,
};

/// 自动权益发放记录 DTO
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoBenefitGrantDto {
    pub id: i64,
    pub user_id: String,
    pub rule_id: i64,
    pub rule_name: Option<String>,
    pub trigger_badge_id: i64,
    pub trigger_badge_name: Option<String>,
    pub benefit_grant_id: Option<i64>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 自动权益发放记录数据库行
#[derive(sqlx::FromRow)]
struct AutoBenefitGrantRow {
    id: i64,
    user_id: String,
    rule_id: i64,
    rule_name: Option<String>,
    trigger_badge_id: i64,
    trigger_badge_name: Option<String>,
    benefit_grant_id: Option<i64>,
    status: String,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<AutoBenefitGrantRow> for AutoBenefitGrantDto {
    fn from(row: AutoBenefitGrantRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            rule_id: row.rule_id,
            rule_name: row.rule_name,
            trigger_badge_id: row.trigger_badge_id,
            trigger_badge_name: row.trigger_badge_name,
            benefit_grant_id: row.benefit_grant_id,
            status: row.status,
            error_message: row.error_message,
            created_at: row.created_at,
            completed_at: row.completed_at,
        }
    }
}

/// 评估日志 DTO
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationLogDto {
    pub id: i64,
    pub user_id: String,
    pub trigger_badge_id: i64,
    pub trigger_badge_name: Option<String>,
    pub evaluation_context: serde_json::Value,
    pub rules_evaluated: i32,
    pub rules_matched: i32,
    pub grants_created: i32,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
}

/// 评估日志数据库行
#[derive(sqlx::FromRow)]
struct EvaluationLogRow {
    id: i64,
    user_id: String,
    trigger_badge_id: i64,
    trigger_badge_name: Option<String>,
    evaluation_context: serde_json::Value,
    rules_evaluated: i32,
    rules_matched: i32,
    grants_created: i32,
    duration_ms: i64,
    created_at: DateTime<Utc>,
}

impl From<EvaluationLogRow> for EvaluationLogDto {
    fn from(row: EvaluationLogRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            trigger_badge_id: row.trigger_badge_id,
            trigger_badge_name: row.trigger_badge_name,
            evaluation_context: row.evaluation_context,
            rules_evaluated: row.rules_evaluated,
            rules_matched: row.rules_matched,
            grants_created: row.grants_created,
            duration_ms: row.duration_ms,
            created_at: row.created_at,
        }
    }
}

/// 自动权益发放记录查询过滤器
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoBenefitGrantFilter {
    pub user_id: Option<String>,
    pub rule_id: Option<i64>,
    pub status: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 评估日志查询过滤器
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationLogFilter {
    pub user_id: Option<String>,
    pub trigger_badge_id: Option<i64>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 获取自动权益发放记录列表
///
/// GET /api/admin/auto-benefits/grants
///
/// 查询自动权益发放记录，支持按用户、规则、状态筛选
pub async fn list_auto_benefit_grants(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<AutoBenefitGrantFilter>,
) -> Result<Json<ApiResponse<PageResponse<AutoBenefitGrantDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 查询总数
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM auto_benefit_grants g
        WHERE ($1::text IS NULL OR g.user_id = $1)
          AND ($2::bigint IS NULL OR g.rule_id = $2)
          AND ($3::text IS NULL OR g.status = $3)
          AND ($4::timestamptz IS NULL OR g.created_at >= $4)
          AND ($5::timestamptz IS NULL OR g.created_at <= $5)
        "#,
    )
    .bind(&filter.user_id)
    .bind(filter.rule_id)
    .bind(&filter.status)
    .bind(filter.start_time)
    .bind(filter.end_time)
    .fetch_one(&state.pool)
    .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    // 查询数据
    let rows = sqlx::query_as::<_, AutoBenefitGrantRow>(
        r#"
        SELECT
            g.id,
            g.user_id,
            g.rule_id,
            r.name as rule_name,
            g.trigger_badge_id,
            b.name as trigger_badge_name,
            g.benefit_grant_id,
            g.status,
            g.error_message,
            g.created_at,
            g.completed_at
        FROM auto_benefit_grants g
        LEFT JOIN badge_redemption_rules r ON r.id = g.rule_id
        LEFT JOIN badges b ON b.id = g.trigger_badge_id
        WHERE ($1::text IS NULL OR g.user_id = $1)
          AND ($2::bigint IS NULL OR g.rule_id = $2)
          AND ($3::text IS NULL OR g.status = $3)
          AND ($4::timestamptz IS NULL OR g.created_at >= $4)
          AND ($5::timestamptz IS NULL OR g.created_at <= $5)
        ORDER BY g.created_at DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(&filter.user_id)
    .bind(filter.rule_id)
    .bind(&filter.status)
    .bind(filter.start_time)
    .bind(filter.end_time)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<AutoBenefitGrantDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取评估日志列表
///
/// GET /api/admin/auto-benefits/logs
///
/// 查询自动权益评估日志，用于调试和审计
pub async fn list_evaluation_logs(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<EvaluationLogFilter>,
) -> Result<Json<ApiResponse<PageResponse<EvaluationLogDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 查询总数
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM auto_benefit_evaluation_logs l
        WHERE ($1::text IS NULL OR l.user_id = $1)
          AND ($2::bigint IS NULL OR l.trigger_badge_id = $2)
          AND ($3::timestamptz IS NULL OR l.created_at >= $3)
          AND ($4::timestamptz IS NULL OR l.created_at <= $4)
        "#,
    )
    .bind(&filter.user_id)
    .bind(filter.trigger_badge_id)
    .bind(filter.start_time)
    .bind(filter.end_time)
    .fetch_one(&state.pool)
    .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    // 查询数据
    let rows = sqlx::query_as::<_, EvaluationLogRow>(
        r#"
        SELECT
            l.id,
            l.user_id,
            l.trigger_badge_id,
            b.name as trigger_badge_name,
            l.evaluation_context,
            l.rules_evaluated,
            l.rules_matched,
            l.grants_created,
            l.duration_ms,
            l.created_at
        FROM auto_benefit_evaluation_logs l
        LEFT JOIN badges b ON b.id = l.trigger_badge_id
        WHERE ($1::text IS NULL OR l.user_id = $1)
          AND ($2::bigint IS NULL OR l.trigger_badge_id = $2)
          AND ($3::timestamptz IS NULL OR l.created_at >= $3)
          AND ($4::timestamptz IS NULL OR l.created_at <= $4)
        ORDER BY l.created_at DESC
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(&filter.user_id)
    .bind(filter.trigger_badge_id)
    .bind(filter.start_time)
    .bind(filter.end_time)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<EvaluationLogDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 重试失败的自动权益发放
///
/// POST /api/admin/auto-benefits/grants/{id}/retry
///
/// 将失败状态的记录重置为 PENDING，让 worker 重新处理
pub async fn retry_auto_grant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<AutoBenefitGrantDto>>, AdminError> {
    // 检查记录是否存在且状态为 FAILED
    let grant: Option<(String,)> =
        sqlx::query_as("SELECT status FROM auto_benefit_grants WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;

    let grant = grant.ok_or_else(|| AdminError::NotFound(format!("自动权益记录不存在: {}", id)))?;

    if grant.0 != "FAILED" {
        return Err(AdminError::Validation(format!(
            "只有失败状态的记录可以重试，当前状态: {}",
            grant.0
        )));
    }

    // 重置状态为 PENDING
    let now = Utc::now();
    let row = sqlx::query_as::<_, AutoBenefitGrantRow>(
        r#"
        UPDATE auto_benefit_grants
        SET status = 'PENDING', error_message = NULL, completed_at = NULL, updated_at = $2
        WHERE id = $1
        RETURNING
            id, user_id, rule_id, NULL::text as rule_name,
            trigger_badge_id, NULL::text as trigger_badge_name,
            benefit_grant_id, status, error_message, created_at, completed_at
        "#,
    )
    .bind(id)
    .bind(now)
    .fetch_one(&state.pool)
    .await?;

    info!(id = id, "Auto benefit grant retry triggered");
    Ok(Json(ApiResponse::success(row.into())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_benefit_grant_filter_default() {
        let filter = AutoBenefitGrantFilter::default();
        assert!(filter.user_id.is_none());
        assert!(filter.rule_id.is_none());
        assert!(filter.status.is_none());
    }

    #[test]
    fn test_evaluation_log_filter_default() {
        let filter = EvaluationLogFilter::default();
        assert!(filter.user_id.is_none());
        assert!(filter.trigger_badge_id.is_none());
    }
}
