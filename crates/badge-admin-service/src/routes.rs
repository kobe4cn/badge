//! 路由配置模块
//!
//! 定义所有 REST API 端点的路由映射

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::{handlers, state::AppState};

/// 构建徽章管理相关的路由
///
/// 包含分类、系列、徽章的 CRUD 操作路由
pub fn badge_routes() -> Router<AppState> {
    Router::new()
        // 分类管理
        .route("/categories", post(handlers::category::create_category))
        .route("/categories", get(handlers::category::list_categories))
        .route("/categories/all", get(handlers::category::list_all_categories))
        .route("/categories/{id}", get(handlers::category::get_category))
        .route("/categories/{id}", put(handlers::category::update_category))
        .route(
            "/categories/{id}",
            delete(handlers::category::delete_category),
        )
        // 系列管理
        .route("/series", post(handlers::series::create_series))
        .route("/series", get(handlers::series::list_series))
        .route("/series/all", get(handlers::series::list_all_series))
        .route("/series/{id}", get(handlers::series::get_series))
        .route("/series/{id}", put(handlers::series::update_series))
        .route("/series/{id}", delete(handlers::series::delete_series))
        // 徽章管理
        .route("/badges", post(handlers::badge::create_badge))
        .route("/badges", get(handlers::badge::list_badges))
        .route("/badges/{id}", get(handlers::badge::get_badge))
        .route("/badges/{id}", put(handlers::badge::update_badge))
        .route("/badges/{id}", delete(handlers::badge::delete_badge))
        .route("/badges/{id}/publish", post(handlers::badge::publish_badge))
        .route("/badges/{id}/offline", post(handlers::badge::offline_badge))
        // 依赖关系管理
        .route(
            "/badges/{badge_id}/dependencies",
            post(handlers::dependency::create_dependency),
        )
        .route(
            "/badges/{badge_id}/dependencies",
            get(handlers::dependency::list_dependencies),
        )
        .route(
            "/dependencies/{id}",
            delete(handlers::dependency::delete_dependency),
        )
}

/// 构建缓存管理路由
///
/// 包含缓存刷新等运维操作
fn cache_routes() -> Router<AppState> {
    Router::new().route(
        "/cache/dependencies/refresh",
        post(handlers::dependency::refresh_dependency_cache),
    )
}

/// 构建规则管理路由
///
/// 包含规则 CRUD、发布和测试操作
fn rule_routes() -> Router<AppState> {
    Router::new()
        .route("/rules", post(handlers::rule::create_rule))
        .route("/rules", get(handlers::rule::list_rules))
        .route("/rules/{id}", get(handlers::rule::get_rule))
        .route("/rules/{id}", put(handlers::rule::update_rule))
        .route("/rules/{id}", delete(handlers::rule::delete_rule))
        .route("/rules/{id}/publish", post(handlers::rule::publish_rule))
        .route("/rules/{id}/test", post(handlers::rule::test_rule))
}

/// 构建发放管理路由
///
/// 包含手动发放、批量发放和发放记录查询
fn grant_routes() -> Router<AppState> {
    Router::new()
        .route("/grants/manual", post(handlers::grant::manual_grant))
        .route("/grants/batch", post(handlers::grant::batch_grant))
        .route("/grants", get(handlers::grant::list_grants))
}

/// 构建取消管理路由
///
/// 包含手动取消、批量取消和取消记录查询
fn revoke_routes() -> Router<AppState> {
    Router::new()
        .route("/revokes/manual", post(handlers::revoke::manual_revoke))
        .route("/revokes/batch", post(handlers::revoke::batch_revoke))
        .route("/revokes", get(handlers::revoke::list_revokes))
}

/// 构建统计报表路由
///
/// 包含总览、今日统计、趋势、排行、类型分布和单徽章统计
fn stats_routes() -> Router<AppState> {
    Router::new()
        .route("/stats/overview", get(handlers::stats::get_overview))
        .route("/stats/today", get(handlers::stats::get_today_stats))
        .route("/stats/trends", get(handlers::stats::get_trends))
        .route("/stats/ranking", get(handlers::stats::get_ranking))
        .route(
            "/stats/distribution/types",
            get(handlers::stats::get_type_distribution),
        )
        .route("/stats/badges/{id}", get(handlers::stats::get_badge_stats))
}

/// 构建会员视图路由
///
/// 包含用户搜索、详情、徽章、兑换记录、统计和账本流水
fn user_view_routes() -> Router<AppState> {
    Router::new()
        .route("/users/search", get(handlers::user_view::search_users))
        .route(
            "/users/{user_id}",
            get(handlers::user_view::get_user_detail),
        )
        .route(
            "/users/{user_id}/badges",
            get(handlers::user_view::get_user_badges),
        )
        .route(
            "/users/{user_id}/redemptions",
            get(handlers::user_view::get_user_redemptions),
        )
        .route(
            "/users/{user_id}/stats",
            get(handlers::user_view::get_user_stats),
        )
        .route(
            "/users/{user_id}/ledger",
            get(handlers::user_view::get_user_ledger),
        )
}

/// 构建操作日志路由
fn log_routes() -> Router<AppState> {
    Router::new().route("/logs", get(handlers::operation_log::list_logs))
}

/// 构建批量任务路由
///
/// 包含任务创建、列表查询和详情/进度查询
fn task_routes() -> Router<AppState> {
    Router::new()
        .route("/tasks", post(handlers::batch_task::create_task))
        .route("/tasks", get(handlers::batch_task::list_tasks))
        .route("/tasks/{id}", get(handlers::batch_task::get_task))
}

/// 构建完整的 API 路由
///
/// 包含所有管理后台 API，挂载在 /api/admin 前缀下
pub fn api_routes() -> Router<AppState> {
    let admin_routes = Router::new()
        .merge(badge_routes())
        .merge(rule_routes())
        .merge(grant_routes())
        .merge(revoke_routes())
        .merge(stats_routes())
        .merge(user_view_routes())
        .merge(log_routes())
        .merge(task_routes())
        .merge(cache_routes());

    Router::new().nest("/api/admin", admin_routes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_construction() {
        let _badge = badge_routes();
        let _rule = rule_routes();
        let _grant = grant_routes();
        let _revoke = revoke_routes();
        let _stats = stats_routes();
        let _user_view = user_view_routes();
        let _log = log_routes();
        let _task = task_routes();
        let _cache = cache_routes();
        let _api = api_routes();
    }
}
