//! 路由配置模块
//!
//! 定义所有 REST API 端点的路由映射，并为每条路由附加 RBAC 权限中间件

use axum::{
    middleware as axum_mw,
    Router,
    routing::{delete, get, patch, post, put},
};

use crate::{handlers, middleware::require_permission, state::AppState};

/// 构建认证相关的路由（公开路由，无需认证和权限检查）
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/me", get(handlers::auth::get_current_user))
        .route("/auth/refresh", post(handlers::auth::refresh_token))
}

/// 构建系统管理路由
///
/// 包含用户、角色、权限管理，每个端点按读/写分别授权
fn system_routes() -> Router<AppState> {
    Router::new()
        // ── 用户管理 — 读 ──
        .route("/system/users", get(handlers::system_user::list_users)
            .layer(axum_mw::from_fn(require_permission("system:user:read"))))
        .route("/system/users/{id}", get(handlers::system_user::get_user)
            .layer(axum_mw::from_fn(require_permission("system:user:read"))))
        // ── 用户管理 — 写 ──
        .route("/system/users", post(handlers::system_user::create_user)
            .layer(axum_mw::from_fn(require_permission("system:user:write"))))
        .route("/system/users/{id}", put(handlers::system_user::update_user)
            .layer(axum_mw::from_fn(require_permission("system:user:write"))))
        .route("/system/users/{id}", delete(handlers::system_user::delete_user)
            .layer(axum_mw::from_fn(require_permission("system:user:write"))))
        .route("/system/users/{id}/reset-password", post(handlers::system_user::reset_password)
            .layer(axum_mw::from_fn(require_permission("system:user:write"))))
        // ── 角色管理 — 读 ──
        .route("/system/roles", get(handlers::system_role::list_roles)
            .layer(axum_mw::from_fn(require_permission("system:role:read"))))
        .route("/system/roles/{id}", get(handlers::system_role::get_role)
            .layer(axum_mw::from_fn(require_permission("system:role:read"))))
        // ── 角色管理 — 写 ──
        .route("/system/roles", post(handlers::system_role::create_role)
            .layer(axum_mw::from_fn(require_permission("system:role:write"))))
        .route("/system/roles/{id}", put(handlers::system_role::update_role)
            .layer(axum_mw::from_fn(require_permission("system:role:write"))))
        .route("/system/roles/{id}", delete(handlers::system_role::delete_role)
            .layer(axum_mw::from_fn(require_permission("system:role:write"))))
        // ── 权限查询 ──
        .route("/system/permissions", get(handlers::system_role::list_permissions)
            .layer(axum_mw::from_fn(require_permission("system:role:read"))))
        .route("/system/permissions/tree", get(handlers::system_role::get_permission_tree)
            .layer(axum_mw::from_fn(require_permission("system:role:read"))))
        // ── API Key — 读 ──
        .route("/system/api-keys", get(handlers::api_key::list_api_keys)
            .layer(axum_mw::from_fn(require_permission("system:apikey:read"))))
        // ── API Key — 写 ──
        .route("/system/api-keys", post(handlers::api_key::create_api_key)
            .layer(axum_mw::from_fn(require_permission("system:apikey:write"))))
        .route("/system/api-keys/{id}", delete(handlers::api_key::delete_api_key)
            .layer(axum_mw::from_fn(require_permission("system:apikey:write"))))
        .route("/system/api-keys/{id}/regenerate", post(handlers::api_key::regenerate_api_key)
            .layer(axum_mw::from_fn(require_permission("system:apikey:write"))))
        .route("/system/api-keys/{id}/status", patch(handlers::api_key::toggle_api_key_status)
            .layer(axum_mw::from_fn(require_permission("system:apikey:write"))))
}

/// 构建徽章管理相关的路由
///
/// 包含分类、系列、徽章的 CRUD 操作路由，按读/写/发布分别授权
pub fn badge_routes() -> Router<AppState> {
    Router::new()
        // ── 分类 — 读 ──
        .route("/categories", get(handlers::category::list_categories)
            .layer(axum_mw::from_fn(require_permission("badge:category:read"))))
        .route("/categories/all", get(handlers::category::list_all_categories)
            .layer(axum_mw::from_fn(require_permission("badge:category:read"))))
        .route("/categories/{id}", get(handlers::category::get_category)
            .layer(axum_mw::from_fn(require_permission("badge:category:read"))))
        // ── 分类 — 写 ──
        .route("/categories", post(handlers::category::create_category)
            .layer(axum_mw::from_fn(require_permission("badge:category:write"))))
        .route("/categories/{id}", put(handlers::category::update_category)
            .layer(axum_mw::from_fn(require_permission("badge:category:write"))))
        .route("/categories/{id}", delete(handlers::category::delete_category)
            .layer(axum_mw::from_fn(require_permission("badge:category:write"))))
        .route("/categories/{id}/status", patch(handlers::category::update_category_status)
            .layer(axum_mw::from_fn(require_permission("badge:category:write"))))
        .route("/categories/{id}/sort", patch(handlers::category::update_category_sort)
            .layer(axum_mw::from_fn(require_permission("badge:category:write"))))
        // ── 系列 — 读 ──
        .route("/series", get(handlers::series::list_series)
            .layer(axum_mw::from_fn(require_permission("badge:series:read"))))
        .route("/series/all", get(handlers::series::list_all_series)
            .layer(axum_mw::from_fn(require_permission("badge:series:read"))))
        .route("/series/{id}", get(handlers::series::get_series)
            .layer(axum_mw::from_fn(require_permission("badge:series:read"))))
        .route("/series/{id}/badges", get(handlers::series::list_series_badges)
            .layer(axum_mw::from_fn(require_permission("badge:series:read"))))
        // ── 系列 — 写 ──
        .route("/series", post(handlers::series::create_series)
            .layer(axum_mw::from_fn(require_permission("badge:series:write"))))
        .route("/series/{id}", put(handlers::series::update_series)
            .layer(axum_mw::from_fn(require_permission("badge:series:write"))))
        .route("/series/{id}", delete(handlers::series::delete_series)
            .layer(axum_mw::from_fn(require_permission("badge:series:write"))))
        .route("/series/{id}/status", patch(handlers::series::update_series_status)
            .layer(axum_mw::from_fn(require_permission("badge:series:write"))))
        .route("/series/{id}/sort", patch(handlers::series::update_series_sort)
            .layer(axum_mw::from_fn(require_permission("badge:series:write"))))
        // ── 徽章 — 读 ──
        .route("/badges", get(handlers::badge::list_badges)
            .layer(axum_mw::from_fn(require_permission("badge:badge:read"))))
        .route("/badges/{id}", get(handlers::badge::get_badge)
            .layer(axum_mw::from_fn(require_permission("badge:badge:read"))))
        // ── 徽章 — 写 ──
        .route("/badges", post(handlers::badge::create_badge)
            .layer(axum_mw::from_fn(require_permission("badge:badge:write"))))
        .route("/badges/{id}", put(handlers::badge::update_badge)
            .layer(axum_mw::from_fn(require_permission("badge:badge:write"))))
        .route("/badges/{id}", delete(handlers::badge::delete_badge)
            .layer(axum_mw::from_fn(require_permission("badge:badge:write"))))
        .route("/badges/{id}/sort", patch(handlers::badge::update_badge_sort)
            .layer(axum_mw::from_fn(require_permission("badge:badge:write"))))
        // ── 徽章 — 发布/下线/归档 ──
        .route("/badges/{id}/publish", post(handlers::badge::publish_badge)
            .layer(axum_mw::from_fn(require_permission("badge:badge:publish"))))
        .route("/badges/{id}/offline", post(handlers::badge::offline_badge)
            .layer(axum_mw::from_fn(require_permission("badge:badge:publish"))))
        .route("/badges/{id}/archive", post(handlers::badge::archive_badge)
            .layer(axum_mw::from_fn(require_permission("badge:badge:publish"))))
        // ── 依赖 — 读 ──
        .route("/badges/{badge_id}/dependencies", get(handlers::dependency::list_dependencies)
            .layer(axum_mw::from_fn(require_permission("badge:dependency:read"))))
        .route("/dependencies/graph", get(handlers::dependency::get_dependency_graph)
            .layer(axum_mw::from_fn(require_permission("badge:dependency:read"))))
        // ── 依赖 — 写 ──
        .route("/badges/{badge_id}/dependencies", post(handlers::dependency::create_dependency)
            .layer(axum_mw::from_fn(require_permission("badge:dependency:write"))))
        .route("/dependencies/{id}", put(handlers::dependency::update_dependency)
            .layer(axum_mw::from_fn(require_permission("badge:dependency:write"))))
        .route("/dependencies/{id}", delete(handlers::dependency::delete_dependency)
            .layer(axum_mw::from_fn(require_permission("badge:dependency:write"))))
}

/// 构建缓存管理路由
///
/// 包含缓存刷新等运维操作
fn cache_routes() -> Router<AppState> {
    Router::new()
        .route("/cache/dependencies/refresh", post(handlers::dependency::refresh_dependency_cache)
            .layer(axum_mw::from_fn(require_permission("badge:dependency:write"))))
        .route("/cache/auto-benefit/refresh", post(handlers::dependency::refresh_auto_benefit_cache)
            .layer(axum_mw::from_fn(require_permission("benefit:benefit:write"))))
}

/// 构建事件类型路由
///
/// 提供事件类型列表查询（用于规则配置）
fn event_type_routes() -> Router<AppState> {
    Router::new()
        .route("/event-types", get(handlers::event_type::list_event_types)
            .layer(axum_mw::from_fn(require_permission("rule:rule:read"))))
}

/// 构建规则管理路由
///
/// 包含规则 CRUD、发布和测试操作
fn rule_routes() -> Router<AppState> {
    Router::new()
        // ── 读 ──
        .route("/rules", get(handlers::rule::list_rules)
            .layer(axum_mw::from_fn(require_permission("rule:rule:read"))))
        .route("/rules/{id}", get(handlers::rule::get_rule)
            .layer(axum_mw::from_fn(require_permission("rule:rule:read"))))
        // ── 写 ──
        .route("/rules", post(handlers::rule::create_rule)
            .layer(axum_mw::from_fn(require_permission("rule:rule:write"))))
        .route("/rules/{id}", put(handlers::rule::update_rule)
            .layer(axum_mw::from_fn(require_permission("rule:rule:write"))))
        .route("/rules/{id}", delete(handlers::rule::delete_rule)
            .layer(axum_mw::from_fn(require_permission("rule:rule:write"))))
        // ── 测试（静态路径在参数路径之前注册，避免 "test" 被 {id} 捕获）──
        .route("/rules/test", post(handlers::rule::test_rule_definition)
            .layer(axum_mw::from_fn(require_permission("rule:rule:test"))))
        .route("/rules/{id}/test", post(handlers::rule::test_rule)
            .layer(axum_mw::from_fn(require_permission("rule:rule:test"))))
        // ── 发布/禁用 ──
        .route("/rules/{id}/publish", post(handlers::rule::publish_rule)
            .layer(axum_mw::from_fn(require_permission("rule:rule:publish"))))
        .route("/rules/{id}/disable", post(handlers::rule::disable_rule)
            .layer(axum_mw::from_fn(require_permission("rule:rule:publish"))))
}

/// 构建发放管理路由
///
/// 包含手动发放、批量发放、发放记录查询、日志查询和导出
fn grant_routes() -> Router<AppState> {
    Router::new()
        // ── 读 ──
        .route("/grants", get(handlers::grant::list_grants)
            .layer(axum_mw::from_fn(require_permission("grant:grant:read"))))
        // export 必须在 {id} 之前注册，避免 "export" 被当作路径参数匹配
        .route("/grants/logs/export", get(handlers::grant::export_grant_logs)
            .layer(axum_mw::from_fn(require_permission("grant:grant:read"))))
        .route("/grants/logs/{id}", get(handlers::grant::get_grant_log_detail)
            .layer(axum_mw::from_fn(require_permission("grant:grant:read"))))
        .route("/grants/logs", get(handlers::grant::list_grant_logs)
            .layer(axum_mw::from_fn(require_permission("grant:grant:read"))))
        .route("/grants/records", get(handlers::grant::list_grant_records)
            .layer(axum_mw::from_fn(require_permission("grant:grant:read"))))
        // ── 写 ──
        .route("/grants/manual", post(handlers::grant::manual_grant)
            .layer(axum_mw::from_fn(require_permission("grant:grant:write"))))
        .route("/grants/batch", post(handlers::grant::batch_grant)
            .layer(axum_mw::from_fn(require_permission("grant:grant:write"))))
        .route("/grants/upload-csv", post(handlers::grant::upload_user_csv)
            .layer(axum_mw::from_fn(require_permission("grant:grant:write"))))
        .route("/grants/preview-filter", post(handlers::grant::preview_user_filter)
            .layer(axum_mw::from_fn(require_permission("grant:grant:write"))))
}

/// 构建取消管理路由
///
/// 包含手动取消、批量取消、自动取消和取消记录查询
fn revoke_routes() -> Router<AppState> {
    Router::new()
        .route("/revokes", get(handlers::revoke::list_revokes)
            .layer(axum_mw::from_fn(require_permission("grant:revoke:read"))))
        .route("/revokes/manual", post(handlers::revoke::manual_revoke)
            .layer(axum_mw::from_fn(require_permission("grant:revoke:write"))))
        .route("/revokes/batch", post(handlers::revoke::batch_revoke)
            .layer(axum_mw::from_fn(require_permission("grant:revoke:write"))))
        // 自动取消（账号注销/身份变更/条件不满足等系统触发场景）
        .route("/revokes/auto", post(handlers::revoke::auto_revoke)
            .layer(axum_mw::from_fn(require_permission("grant:revoke:write"))))
}

/// 构建统计报表路由
///
/// 包含总览、今日统计、趋势、排行、类型分布和单徽章统计
fn stats_routes() -> Router<AppState> {
    Router::new()
        .route("/stats/overview", get(handlers::stats::get_overview)
            .layer(axum_mw::from_fn(require_permission("stats:overview:read"))))
        .route("/stats/today", get(handlers::stats::get_today_stats)
            .layer(axum_mw::from_fn(require_permission("stats:overview:read"))))
        .route("/stats/trends", get(handlers::stats::get_trends)
            .layer(axum_mw::from_fn(require_permission("stats:overview:read"))))
        .route("/stats/trend/activity", get(handlers::stats::get_activity_trend)
            .layer(axum_mw::from_fn(require_permission("stats:overview:read"))))
        .route("/stats/ranking", get(handlers::stats::get_ranking)
            .layer(axum_mw::from_fn(require_permission("stats:overview:read"))))
        .route("/stats/distribution/types", get(handlers::stats::get_type_distribution)
            .layer(axum_mw::from_fn(require_permission("stats:overview:read"))))
        .route("/stats/badges/{id}", get(handlers::stats::get_badge_stats)
            .layer(axum_mw::from_fn(require_permission("stats:overview:read"))))
}

/// 构建会员视图路由
///
/// 包含用户搜索、详情、徽章、兑换记录、统计、账本流水和权益
fn user_view_routes() -> Router<AppState> {
    Router::new()
        .route("/users/search", get(handlers::user_view::search_users)
            .layer(axum_mw::from_fn(require_permission("user:view:read"))))
        .route("/users/{user_id}", get(handlers::user_view::get_user_detail)
            .layer(axum_mw::from_fn(require_permission("user:view:read"))))
        .route("/users/{user_id}/badges", get(handlers::user_view::get_user_badges)
            .layer(axum_mw::from_fn(require_permission("user:badge:read"))))
        .route("/users/{user_id}/redemptions", get(handlers::user_view::get_user_redemptions)
            .layer(axum_mw::from_fn(require_permission("user:view:read"))))
        .route("/users/{user_id}/stats", get(handlers::user_view::get_user_stats)
            .layer(axum_mw::from_fn(require_permission("user:view:read"))))
        .route("/users/{user_id}/ledger", get(handlers::user_view::get_user_ledger)
            .layer(axum_mw::from_fn(require_permission("user:view:read"))))
        .route("/users/{user_id}/benefits", get(handlers::benefit::get_user_benefits)
            .layer(axum_mw::from_fn(require_permission("user:view:read"))))
        .route("/users/{user_id}/redemption-history", get(handlers::redemption::get_user_redemption_history)
            .layer(axum_mw::from_fn(require_permission("user:view:read"))))
}

/// 构建操作日志路由
fn log_routes() -> Router<AppState> {
    Router::new()
        .route("/logs", get(handlers::operation_log::list_logs)
            .layer(axum_mw::from_fn(require_permission("log:operation:read"))))
}

/// 构建批量任务路由
///
/// 包含任务创建、列表查询、详情/进度查询、取消和结果下载
fn task_routes() -> Router<AppState> {
    Router::new()
        // ── 读 ──
        .route("/tasks", get(handlers::batch_task::list_tasks)
            .layer(axum_mw::from_fn(require_permission("grant:task:read"))))
        .route("/tasks/{id}", get(handlers::batch_task::get_task)
            .layer(axum_mw::from_fn(require_permission("grant:task:read"))))
        .route("/tasks/{id}/failures", get(handlers::batch_task::get_task_failures)
            .layer(axum_mw::from_fn(require_permission("grant:task:read"))))
        // 下载失败清单需要放在 /failures 之后避免路由冲突
        .route("/tasks/{id}/failures/download", get(handlers::batch_task::download_task_failures)
            .layer(axum_mw::from_fn(require_permission("grant:task:read"))))
        .route("/tasks/{id}/result", get(handlers::batch_task::get_task_result)
            .layer(axum_mw::from_fn(require_permission("grant:task:read"))))
        // ── 写 ──
        .route("/tasks", post(handlers::batch_task::create_task)
            .layer(axum_mw::from_fn(require_permission("grant:task:write"))))
        .route("/tasks/{id}/cancel", post(handlers::batch_task::cancel_task)
            .layer(axum_mw::from_fn(require_permission("grant:task:write"))))
        .route("/tasks/{id}/retry", post(handlers::batch_task::trigger_task_retry)
            .layer(axum_mw::from_fn(require_permission("grant:task:write"))))
}

/// 构建模板管理路由
///
/// 包含模板列表、详情、预览和从模板创建规则
fn template_routes() -> Router<AppState> {
    Router::new()
        .route("/templates", get(handlers::template::list_templates)
            .layer(axum_mw::from_fn(require_permission("rule:template:read"))))
        .route("/templates/{code}", get(handlers::template::get_template)
            .layer(axum_mw::from_fn(require_permission("rule:template:read"))))
        .route("/templates/{code}/preview", post(handlers::template::preview_template)
            .layer(axum_mw::from_fn(require_permission("rule:template:read"))))
        // 从模板创建规则需要规则写权限
        .route("/rules/from-template", post(handlers::template::create_rule_from_template)
            .layer(axum_mw::from_fn(require_permission("rule:rule:write"))))
}

/// 构建权益管理路由
///
/// 包含权益的 CRUD 操作、发放记录查询、同步和用户权益查询
fn benefit_routes() -> Router<AppState> {
    Router::new()
        // ── 读 ──
        .route("/benefits", get(handlers::benefit::list_benefits)
            .layer(axum_mw::from_fn(require_permission("benefit:benefit:read"))))
        .route("/benefits/{id}", get(handlers::benefit::get_benefit)
            .layer(axum_mw::from_fn(require_permission("benefit:benefit:read"))))
        .route("/benefits/sync-logs", get(handlers::benefit::list_sync_logs)
            .layer(axum_mw::from_fn(require_permission("benefit:benefit:read"))))
        .route("/benefit-grants", get(handlers::benefit::list_benefit_grants)
            .layer(axum_mw::from_fn(require_permission("benefit:grant:read"))))
        // ── 写 ──
        .route("/benefits", post(handlers::benefit::create_benefit)
            .layer(axum_mw::from_fn(require_permission("benefit:benefit:write"))))
        .route("/benefits/{id}", put(handlers::benefit::update_benefit)
            .layer(axum_mw::from_fn(require_permission("benefit:benefit:write"))))
        .route("/benefits/{id}", delete(handlers::benefit::delete_benefit)
            .layer(axum_mw::from_fn(require_permission("benefit:benefit:write"))))
        .route("/benefits/{id}/link-badge", post(handlers::benefit::link_badge_to_benefit)
            .layer(axum_mw::from_fn(require_permission("benefit:benefit:write"))))
        .route("/benefits/sync", post(handlers::benefit::trigger_sync)
            .layer(axum_mw::from_fn(require_permission("benefit:benefit:write"))))
}

/// 构建通知配置管理路由
///
/// 包含通知配置的 CRUD、测试发送和任务查询
fn notification_routes() -> Router<AppState> {
    Router::new()
        // ── 读 ──
        .route("/notification-configs", get(handlers::notification::list_notification_configs)
            .layer(axum_mw::from_fn(require_permission("notification:config:read"))))
        .route("/notification-configs/{id}", get(handlers::notification::get_notification_config)
            .layer(axum_mw::from_fn(require_permission("notification:config:read"))))
        .route("/notification-tasks", get(handlers::notification::list_notification_tasks)
            .layer(axum_mw::from_fn(require_permission("notification:task:read"))))
        // ── 写 ──
        .route("/notification-configs", post(handlers::notification::create_notification_config)
            .layer(axum_mw::from_fn(require_permission("notification:config:write"))))
        .route("/notification-configs/{id}", put(handlers::notification::update_notification_config)
            .layer(axum_mw::from_fn(require_permission("notification:config:write"))))
        .route("/notification-configs/{id}", delete(handlers::notification::delete_notification_config)
            .layer(axum_mw::from_fn(require_permission("notification:config:write"))))
        // ── 测试 ──
        .route("/notification-configs/test", post(handlers::notification::test_notification)
            .layer(axum_mw::from_fn(require_permission("notification:config:write"))))
}

/// 构建兑换管理路由
///
/// 包含兑换规则的 CRUD 和执行兑换操作
fn redemption_routes() -> Router<AppState> {
    Router::new()
        // ── 读 ──
        .route("/redemption/rules", get(handlers::redemption::list_redemption_rules)
            .layer(axum_mw::from_fn(require_permission("benefit:redemption:read"))))
        .route("/redemption/rules/{id}", get(handlers::redemption::get_redemption_rule)
            .layer(axum_mw::from_fn(require_permission("benefit:redemption:read"))))
        .route("/redemption/orders", get(handlers::redemption::list_redemption_orders)
            .layer(axum_mw::from_fn(require_permission("benefit:redemption:read"))))
        .route("/redemption/orders/{order_no}", get(handlers::redemption::get_redemption_order)
            .layer(axum_mw::from_fn(require_permission("benefit:redemption:read"))))
        // ── 写 ──
        .route("/redemption/rules", post(handlers::redemption::create_redemption_rule)
            .layer(axum_mw::from_fn(require_permission("benefit:redemption:write"))))
        .route("/redemption/rules/{id}", put(handlers::redemption::update_redemption_rule)
            .layer(axum_mw::from_fn(require_permission("benefit:redemption:write"))))
        .route("/redemption/rules/{id}", delete(handlers::redemption::delete_redemption_rule)
            .layer(axum_mw::from_fn(require_permission("benefit:redemption:write"))))
        .route("/redemption/redeem", post(handlers::redemption::redeem)
            .layer(axum_mw::from_fn(require_permission("benefit:redemption:write"))))
}

/// 构建自动权益管理路由
///
/// 包含自动权益发放记录和评估日志的查询
fn auto_benefit_routes() -> Router<AppState> {
    Router::new()
        // ── 读 ──
        .route("/auto-benefits/grants", get(handlers::auto_benefit::list_auto_benefit_grants)
            .layer(axum_mw::from_fn(require_permission("benefit:auto:read"))))
        .route("/auto-benefits/logs", get(handlers::auto_benefit::list_evaluation_logs)
            .layer(axum_mw::from_fn(require_permission("benefit:auto:read"))))
        // ── 写 ──
        .route("/auto-benefits/grants/{id}/retry", post(handlers::auto_benefit::retry_auto_grant)
            .layer(axum_mw::from_fn(require_permission("benefit:auto:write"))))
}

/// 构建素材库管理路由
///
/// 包含素材的 CRUD 操作，支持图片、动画、视频和 3D 模型
fn asset_routes() -> Router<AppState> {
    Router::new()
        // ── 读 ──
        .route("/assets", get(handlers::asset::list_assets)
            .layer(axum_mw::from_fn(require_permission("asset:read"))))
        .route("/assets/categories", get(handlers::asset::list_categories)
            .layer(axum_mw::from_fn(require_permission("asset:read"))))
        .route("/assets/{id}", get(handlers::asset::get_asset)
            .layer(axum_mw::from_fn(require_permission("asset:read"))))
        // ── 写 ──
        .route("/assets", post(handlers::asset::create_asset)
            .layer(axum_mw::from_fn(require_permission("asset:write"))))
        .route("/assets/{id}", put(handlers::asset::update_asset)
            .layer(axum_mw::from_fn(require_permission("asset:write"))))
        .route("/assets/{id}", delete(handlers::asset::delete_asset)
            .layer(axum_mw::from_fn(require_permission("asset:write"))))
        .route("/assets/{id}/use", post(handlers::asset::increment_usage)
            .layer(axum_mw::from_fn(require_permission("asset:write"))))
}

/// 构建完整的 API 路由
///
/// 返回所有管理后台 API 路由（不含前缀，由调用方在 main.rs 中挂载）
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(auth_routes())
        .merge(system_routes())
        .merge(badge_routes())
        .merge(event_type_routes())
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
        .merge(notification_routes())
        .merge(auto_benefit_routes())
        .merge(asset_routes())
}

/// 构建外部 API 路由（供第三方系统调用，API Key 认证）
///
/// 与管理后台 `/api/admin/` 分离，使用 `/api/v1/` 前缀。
/// 暴露只读查询和兑换操作接口。
/// 每条路由均附加细粒度权限校验，权限检查在 API Key 认证之后执行。
pub fn external_api_routes(external_state: crate::middleware::ExternalApiState) -> Router<AppState> {
    use crate::middleware::{api_key_auth_middleware, require_api_key_permission};

    Router::new()
        // 用户徽章查询 — 只读
        .route(
            "/users/{user_id}/badges",
            get(handlers::user_view::get_user_badges)
                .layer(axum_mw::from_fn(require_api_key_permission("read:badges"))),
        )
        .route(
            "/users/{user_id}/stats",
            get(handlers::user_view::get_user_stats)
                .layer(axum_mw::from_fn(require_api_key_permission("read:users"))),
        )
        // 兑换操作 — 写
        .route(
            "/redemption/redeem",
            post(handlers::redemption::redeem)
                .layer(axum_mw::from_fn(require_api_key_permission("write:redemption"))),
        )
        .route(
            "/redemption/orders/{order_no}",
            get(handlers::redemption::get_redemption_order)
                .layer(axum_mw::from_fn(require_api_key_permission("read:redemption"))),
        )
        // 发放记录查询 — 只读
        .route(
            "/grants/logs",
            get(handlers::grant::list_grant_logs)
                .layer(axum_mw::from_fn(require_api_key_permission("read:grants"))),
        )
        // API Key 认证 + 限流层（外层，先执行认证和限流再进入权限检查）
        .layer(axum::middleware::from_fn_with_state(
            external_state,
            api_key_auth_middleware,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_construction() {
        // 验证所有路由模块可以正确构建且合并无冲突
        let routes = vec![
            auth_routes(),
            system_routes(),
            badge_routes(),
            event_type_routes(),
            rule_routes(),
            grant_routes(),
            revoke_routes(),
            stats_routes(),
            user_view_routes(),
            log_routes(),
            task_routes(),
            cache_routes(),
            template_routes(),
            benefit_routes(),
            redemption_routes(),
            notification_routes(),
        ];

        assert_eq!(routes.len(), 16, "应包含 16 个路由模块");

        let combined = routes
            .into_iter()
            .fold(axum::Router::new(), |router, r| router.merge(r));
        drop(combined);
    }
}
