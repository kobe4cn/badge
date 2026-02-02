//! 路由配置模块
//!
//! 定义所有 REST API 端点的路由映射

use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};

use crate::{handlers, state::AppState};

/// 构建认证相关的路由（公开路由，无需认证）
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/me", get(handlers::auth::get_current_user))
        .route("/auth/refresh", post(handlers::auth::refresh_token))
}

/// 构建系统管理路由
///
/// 包含用户、角色、权限管理
fn system_routes() -> Router<AppState> {
    Router::new()
        // 用户管理
        .route("/system/users", get(handlers::system_user::list_users))
        .route("/system/users", post(handlers::system_user::create_user))
        .route("/system/users/{id}", get(handlers::system_user::get_user))
        .route("/system/users/{id}", put(handlers::system_user::update_user))
        .route(
            "/system/users/{id}",
            delete(handlers::system_user::delete_user),
        )
        .route(
            "/system/users/{id}/reset-password",
            post(handlers::system_user::reset_password),
        )
        // 角色管理
        .route("/system/roles", get(handlers::system_role::list_roles))
        .route("/system/roles", post(handlers::system_role::create_role))
        .route("/system/roles/{id}", get(handlers::system_role::get_role))
        .route("/system/roles/{id}", put(handlers::system_role::update_role))
        .route(
            "/system/roles/{id}",
            delete(handlers::system_role::delete_role),
        )
        // 权限管理
        .route(
            "/system/permissions",
            get(handlers::system_role::list_permissions),
        )
        .route(
            "/system/permissions/tree",
            get(handlers::system_role::get_permission_tree),
        )
}

/// 构建徽章管理相关的路由
///
/// 包含分类、系列、徽章的 CRUD 操作路由
pub fn badge_routes() -> Router<AppState> {
    Router::new()
        // 分类管理
        .route("/categories", post(handlers::category::create_category))
        .route("/categories", get(handlers::category::list_categories))
        .route(
            "/categories/all",
            get(handlers::category::list_all_categories),
        )
        .route("/categories/{id}", get(handlers::category::get_category))
        .route("/categories/{id}", put(handlers::category::update_category))
        .route(
            "/categories/{id}",
            delete(handlers::category::delete_category),
        )
        .route(
            "/categories/{id}/status",
            patch(handlers::category::update_category_status),
        )
        .route(
            "/categories/{id}/sort",
            patch(handlers::category::update_category_sort),
        )
        // 系列管理
        .route("/series", post(handlers::series::create_series))
        .route("/series", get(handlers::series::list_series))
        .route("/series/all", get(handlers::series::list_all_series))
        .route("/series/{id}", get(handlers::series::get_series))
        .route("/series/{id}", put(handlers::series::update_series))
        .route("/series/{id}", delete(handlers::series::delete_series))
        .route(
            "/series/{id}/status",
            patch(handlers::series::update_series_status),
        )
        .route(
            "/series/{id}/sort",
            patch(handlers::series::update_series_sort),
        )
        .route(
            "/series/{id}/badges",
            get(handlers::series::list_series_badges),
        )
        // 徽章管理
        .route("/badges", post(handlers::badge::create_badge))
        .route("/badges", get(handlers::badge::list_badges))
        .route("/badges/{id}", get(handlers::badge::get_badge))
        .route("/badges/{id}", put(handlers::badge::update_badge))
        .route("/badges/{id}", delete(handlers::badge::delete_badge))
        .route("/badges/{id}/publish", post(handlers::badge::publish_badge))
        .route("/badges/{id}/offline", post(handlers::badge::offline_badge))
        .route("/badges/{id}/archive", post(handlers::badge::archive_badge))
        .route(
            "/badges/{id}/sort",
            patch(handlers::badge::update_badge_sort),
        )
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
            put(handlers::dependency::update_dependency),
        )
        .route(
            "/dependencies/{id}",
            delete(handlers::dependency::delete_dependency),
        )
        .route(
            "/dependencies/graph",
            get(handlers::dependency::get_dependency_graph),
        )
}

/// 构建缓存管理路由
///
/// 包含缓存刷新等运维操作
fn cache_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/cache/dependencies/refresh",
            post(handlers::dependency::refresh_dependency_cache),
        )
        .route(
            "/cache/auto-benefit/refresh",
            post(handlers::dependency::refresh_auto_benefit_cache),
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
        .route(
            "/stats/trend/activity",
            get(handlers::stats::get_activity_trend),
        )
        .route("/stats/ranking", get(handlers::stats::get_ranking))
        .route(
            "/stats/distribution/types",
            get(handlers::stats::get_type_distribution),
        )
        .route("/stats/badges/{id}", get(handlers::stats::get_badge_stats))
}

/// 构建会员视图路由
///
/// 包含用户搜索、详情、徽章、兑换记录、统计、账本流水和权益
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
        .route(
            "/users/{user_id}/benefits",
            get(handlers::benefit::get_user_benefits),
        )
        .route(
            "/users/{user_id}/redemption-history",
            get(handlers::redemption::get_user_redemption_history),
        )
}

/// 构建操作日志路由
fn log_routes() -> Router<AppState> {
    Router::new().route("/logs", get(handlers::operation_log::list_logs))
}

/// 构建批量任务路由
///
/// 包含任务创建、列表查询、详情/进度查询、取消和结果下载
fn task_routes() -> Router<AppState> {
    Router::new()
        .route("/tasks", post(handlers::batch_task::create_task))
        .route("/tasks", get(handlers::batch_task::list_tasks))
        .route("/tasks/{id}", get(handlers::batch_task::get_task))
        .route(
            "/tasks/{id}/cancel",
            post(handlers::batch_task::cancel_task),
        )
        .route(
            "/tasks/{id}/failures",
            get(handlers::batch_task::get_task_failures),
        )
        .route(
            "/tasks/{id}/result",
            get(handlers::batch_task::get_task_result),
        )
}

/// 构建模板管理路由
///
/// 包含模板列表、详情、预览和从模板创建规则
fn template_routes() -> Router<AppState> {
    Router::new()
        .route("/templates", get(handlers::template::list_templates))
        .route("/templates/{code}", get(handlers::template::get_template))
        .route(
            "/templates/{code}/preview",
            post(handlers::template::preview_template),
        )
        .route(
            "/rules/from-template",
            post(handlers::template::create_rule_from_template),
        )
}

/// 构建权益管理路由
///
/// 包含权益的 CRUD 操作、发放记录查询、同步和用户权益查询
fn benefit_routes() -> Router<AppState> {
    Router::new()
        // 权益 CRUD
        .route("/benefits", post(handlers::benefit::create_benefit))
        .route("/benefits", get(handlers::benefit::list_benefits))
        .route("/benefits/{id}", get(handlers::benefit::get_benefit))
        .route("/benefits/{id}", put(handlers::benefit::update_benefit))
        .route("/benefits/{id}", delete(handlers::benefit::delete_benefit))
        // 权益关联徽章
        .route(
            "/benefits/{id}/link-badge",
            post(handlers::benefit::link_badge_to_benefit),
        )
        // 权益同步
        .route(
            "/benefits/sync-logs",
            get(handlers::benefit::list_sync_logs),
        )
        .route("/benefits/sync", post(handlers::benefit::trigger_sync))
        // 权益发放记录
        .route(
            "/benefit-grants",
            get(handlers::benefit::list_benefit_grants),
        )
}

/// 构建兑换管理路由
///
/// 包含兑换规则的 CRUD 和执行兑换操作
fn redemption_routes() -> Router<AppState> {
    Router::new()
        // 兑换规则管理
        .route(
            "/redemption/rules",
            post(handlers::redemption::create_redemption_rule),
        )
        .route(
            "/redemption/rules",
            get(handlers::redemption::list_redemption_rules),
        )
        .route(
            "/redemption/rules/{id}",
            get(handlers::redemption::get_redemption_rule),
        )
        .route(
            "/redemption/rules/{id}",
            put(handlers::redemption::update_redemption_rule),
        )
        .route(
            "/redemption/rules/{id}",
            delete(handlers::redemption::delete_redemption_rule),
        )
        // 执行兑换
        .route("/redemption/redeem", post(handlers::redemption::redeem))
        // 兑换订单查询
        .route(
            "/redemption/orders",
            get(handlers::redemption::list_redemption_orders),
        )
}

/// 构建完整的 API 路由
///
/// 返回所有管理后台 API 路由（不含前缀，由调用方在 main.rs 中挂载）
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(auth_routes())
        .merge(system_routes())
        .merge(badge_routes())
        .merge(rule_routes())
        .merge(grant_routes())
        .merge(revoke_routes())
        .merge(stats_routes())
        .merge(user_view_routes())
        .merge(log_routes())
        .merge(task_routes())
        .merge(cache_routes())
        .merge(template_routes())
        .merge(benefit_routes())
        .merge(redemption_routes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_construction() {
        let _auth = auth_routes();
        let _system = system_routes();
        let _badge = badge_routes();
        let _rule = rule_routes();
        let _grant = grant_routes();
        let _revoke = revoke_routes();
        let _stats = stats_routes();
        let _user_view = user_view_routes();
        let _log = log_routes();
        let _task = task_routes();
        let _cache = cache_routes();
        let _template = template_routes();
        let _benefit = benefit_routes();
        let _redemption = redemption_routes();
        let _api = api_routes();
    }
}
