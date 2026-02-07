//! 徽章管理后台服务（B端）
//!
//! 提供徽章配置、发放管理、统计报表等 REST API。

use std::sync::Arc;

use axum::{Json, Router, extract::Request, http::HeaderValue, middleware, middleware::Next, response::Response, routing::get};
use badge_admin_service::{auth::JwtConfig, middleware::{auth_middleware, audit_middleware}, routes, state::AppState};
use badge_proto::badge::badge_management_service_client::BadgeManagementServiceClient;
use badge_proto::rule_engine::rule_engine_service_client::RuleEngineServiceClient;
use badge_shared::{
    cache::Cache,
    config::AppConfig,
    database::Database,
    observability::{self, middleware as obs_middleware},
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 统一加载配置：从 config/{service_name}.toml 加载，包含可观测性配置
    let config = AppConfig::load("badge-admin-service").unwrap_or_default();

    // 从 AppConfig 中提取可观测性配置并注入服务名
    let obs_config = config.observability.clone().with_service_name(&config.service_name);
    let _guard = observability::init(&obs_config).await?;

    info!("Starting badge-admin-service on {}", config.server_addr());

    // 初始化基础设施
    let db = Database::connect(&config.database).await?;
    let cache = Arc::new(Cache::new(&config.redis)?);

    // JWT 密钥配置：生产环境必须通过环境变量注入，开发环境使用默认值
    let jwt_secret = std::env::var("BADGE_JWT_SECRET").unwrap_or_else(|_| {
        let default_secret = "badge-admin-secret-key-change-in-production".to_string();
        if std::env::var("BADGE_ENV").unwrap_or_default() == "production" {
            panic!("BADGE_JWT_SECRET must be set in production environment");
        }
        warn!("Using default JWT secret - set BADGE_JWT_SECRET for production");
        default_secret
    });

    let jwt_expires = std::env::var("BADGE_JWT_EXPIRES_SECS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(86400);

    let jwt_config = JwtConfig {
        secret: jwt_secret,
        expires_in_secs: jwt_expires,
        issuer: "badge-admin-service".to_string(),
    };

    let mut state = AppState::with_jwt_config(db.pool().clone(), cache.clone(), jwt_config);

    // 初始化兑换服务：RedemptionRepository → RedemptionService → 注入 AppState
    let redemption_repo = Arc::new(
        badge_management::RedemptionRepository::new(db.pool().clone()),
    );
    let redemption_service = Arc::new(badge_management::RedemptionService::new(
        redemption_repo,
        cache.clone(),
        db.pool().clone(),
    ));
    state.set_redemption_service(redemption_service);
    info!("RedemptionService initialized");

    // 尝试连接 badge-management-service 的 gRPC 端点（用于跨服务刷新缓存）
    // 默认地址为 http://127.0.0.1:50052，可通过环境变量 BADGE_MANAGEMENT_GRPC_ADDR 覆盖
    let grpc_addr = std::env::var("BADGE_MANAGEMENT_GRPC_ADDR")
        .unwrap_or_else(|_| "http://127.0.0.1:50052".to_string());

    match BadgeManagementServiceClient::connect(grpc_addr.clone()).await {
        Ok(client) => {
            state.set_badge_management_client(client).await;
            info!("Connected to badge-management-service gRPC at {}", grpc_addr);
        }
        Err(e) => {
            // 连接失败不阻止服务启动，只记录警告
            warn!(
                "Failed to connect to badge-management-service gRPC at {}: {}. \
                Dependency cache refresh will be local-only.",
                grpc_addr, e
            );
        }
    }

    // 尝试连接规则引擎 gRPC 端点（用于规则测试和评估）
    // 规则引擎默认端口 50051，与 badge-management-service 的 50052 区分
    let rule_engine_addr = std::env::var("RULE_ENGINE_GRPC_ADDR")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());

    match RuleEngineServiceClient::connect(rule_engine_addr.clone()).await {
        Ok(client) => {
            state.set_rule_engine_client(client).await;
            info!("Connected to rule-engine gRPC at {}", rule_engine_addr);
        }
        Err(e) => {
            // 连接失败不阻止服务启动，规则测试功能将不可用
            warn!(
                "Failed to connect to rule-engine gRPC at {}: {}. \
                Rule testing will be unavailable.",
                rule_engine_addr, e
            );
        }
    }

    // CORS 配置：通过 BADGE_CORS_ORIGINS 环境变量控制允许的来源
    // 默认允许本地开发地址，生产环境应设置为实际域名
    let allowed_origins = std::env::var("BADGE_CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3001,http://localhost:5173".to_string());

    let cors = if allowed_origins == "*" {
        // 生产环境使用通配符 CORS 是严重的安全隐患，可能导致跨站请求伪造
        if std::env::var("BADGE_ENV").unwrap_or_default() == "production" {
            warn!("BADGE_CORS_ORIGINS=\"*\" 在生产环境中不安全，请设置为具体域名");
        }
        info!("CORS allowed_origins: * (all origins)");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        info!("CORS allowed_origins: {}", allowed_origins);
        let origins: Vec<_> = allowed_origins
            .split(',')
            .filter_map(|s| s.trim().parse::<HeaderValue>().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    // 外部 API 路由需要 PgPool（认证查库）和 Cache（限流计数），
    // 在 db 被 move 到 readiness_check 之前提前构造
    let external_state = badge_admin_service::middleware::ExternalApiState {
        pool: db.pool().clone(),
        cache: cache.clone(),
    };

    // 启动批量任务后台 Worker
    // 在 state 被 move 到 Router 之前克隆连接池
    let batch_worker_pool = db.pool().clone();
    tokio::spawn(async move {
        let worker = badge_admin_service::worker::BatchTaskWorker::new(batch_worker_pool);
        worker.run().await;
    });

    // 启动徽章过期处理 Worker
    let expire_worker_pool = db.pool().clone();
    tokio::spawn(async move {
        let worker = badge_admin_service::worker::ExpireWorker::with_defaults(expire_worker_pool);
        worker.run().await;
    });

    // 启动定时任务调度 Worker
    let scheduled_worker_pool = db.pool().clone();
    tokio::spawn(async move {
        let worker = badge_admin_service::worker::ScheduledTaskWorker::new(scheduled_worker_pool);
        worker.run().await;
    });

    let app = Router::new()
        .nest("/api/admin", routes::api_routes())
        .nest("/api/v1", routes::external_api_routes(external_state))
        .route("/health", get(health_check))
        .route(
            "/ready",
            get({
                let db_for_ready = db;
                let cache_for_ready = cache;
                move || readiness_check(db_for_ready.clone(), cache_for_ready.clone())
            }),
        )
        // 审计中间件：自动记录写操作到 operation_logs（位于 auth 之后，可访问 Claims）
        .layer(middleware::from_fn_with_state(state.clone(), audit_middleware))
        // HTTP 安全头：纵深防御，即使反向代理未配置也确保基本安全策略生效
        .layer(middleware::from_fn(security_headers))
        .layer(cors)
        // 认证中间件：验证 JWT Token
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        // 可观测性中间件：请求追踪和指标收集
        .layer(middleware::from_fn(obs_middleware::http_tracing))
        .layer(middleware::from_fn(obs_middleware::request_id))
        .with_state(state);

    let listener = TcpListener::bind(config.server_addr()).await?;
    info!("Listening on {}", config.server_addr());

    // 优雅关闭：收到 SIGTERM（K8s 停止 Pod）或 Ctrl+C 时，
    // 停止接收新连接并等待已有请求处理完毕
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");

    Ok(())
}

/// 为所有响应注入 HTTP 安全头
///
/// 作为纵深防御的一环，确保即使上游反向代理（如 Nginx/Envoy）未正确配置，
/// 应用层仍能提供基本的浏览器安全策略。
async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    // 禁止浏览器猜测 Content-Type，防止将非可执行内容误判为脚本执行
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    // 禁止页面被嵌入 iframe，防止点击劫持攻击
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    // 强制浏览器后续访问只使用 HTTPS，有效期一年且包含子域名
    headers.insert(
        "strict-transport-security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    // 现代浏览器已内置 XSS 过滤，旧的 X-XSS-Protection 反而可能引入侧信道漏洞，
    // 设为 0 显式禁用，安全策略应依赖 CSP（Content-Security-Policy）
    headers.insert("x-xss-protection", "0".parse().unwrap());
    response
}

/// 监听关闭信号
///
/// K8s 通过 SIGTERM 通知 Pod 停止；本地开发通过 Ctrl+C。
/// 收到任一信号后返回，触发 axum 的优雅关闭流程。
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("注册 Ctrl+C 处理器失败");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("注册 SIGTERM 处理器失败")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, initiating graceful shutdown..."),
        _ = terminate => info!("Received SIGTERM, initiating graceful shutdown..."),
    }
}

/// 存活探针：服务进程正常即返回 ok
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "badge-admin-service"
    }))
}

/// 就绪探针：检查数据库和 Redis 连接是否可用
///
/// K8s 就绪探针失败时会将 Pod 从 Service 端点移除，
/// 避免将流量路由到无法正常处理请求的实例。
async fn readiness_check(db: Database, cache: Arc<Cache>) -> Json<serde_json::Value> {
    let db_ok = db.health_check().await.is_ok();
    let cache_ok = cache.health_check().await.is_ok();
    let all_ok = db_ok && cache_ok;

    Json(serde_json::json!({
        "status": if all_ok { "ok" } else { "degraded" },
        "service": "badge-admin-service",
        "checks": {
            "database": if db_ok { "ok" } else { "fail" },
            "redis": if cache_ok { "ok" } else { "fail" }
        }
    }))
}
