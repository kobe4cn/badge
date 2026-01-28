//! 路由配置模块
//!
//! 定义所有 REST API 端点的路由映射

use axum::{
    routing::{delete, get, post, put},
    Router,
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
        .route("/categories/{id}", get(handlers::category::get_category))
        .route("/categories/{id}", put(handlers::category::update_category))
        .route("/categories/{id}", delete(handlers::category::delete_category))
        // 系列管理
        .route("/series", post(handlers::series::create_series))
        .route("/series", get(handlers::series::list_series))
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
}

/// 构建完整的 API 路由
///
/// 包含所有管理后台 API，挂载在 /api/admin 前缀下
pub fn api_routes() -> Router<AppState> {
    Router::new().nest("/api/admin", badge_routes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_construction() {
        // 验证路由可以正常构建
        let _routes = badge_routes();
        let _api_routes = api_routes();
    }
}
