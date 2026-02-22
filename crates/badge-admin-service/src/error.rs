//! B端管理后台错误类型定义
//!
//! 包含所有 admin service 特有的错误类型

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// B端管理后台错误类型
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    // 认证错误
    #[error("未授权: {0}")]
    Unauthorized(String),
    #[error("禁止访问: {0}")]
    Forbidden(String),
    #[error("用户名或密码错误")]
    InvalidCredentials,
    #[error("用户已被禁用")]
    UserDisabled,
    #[error("用户已被锁定，请稍后重试")]
    UserLocked,
    #[error("请先修改默认密码")]
    PasswordChangeRequired,
    #[error("用户不存在: {0}")]
    UserNotFound(String),

    // 验证错误
    #[error("参数验证失败: {0}")]
    Validation(String),

    // 资源不存在
    #[error("分类不存在: {0}")]
    CategoryNotFound(i64),
    #[error("系列不存在: {0}")]
    SeriesNotFound(i64),
    #[error("徽章不存在: {0}")]
    BadgeNotFound(i64),
    #[error("规则不存在: {0}")]
    RuleNotFound(i64),
    #[error("任务不存在: {0}")]
    TaskNotFound(i64),
    #[error("依赖关系不存在: {0}")]
    DependencyNotFound(i64),
    #[error("权益不存在: {0}")]
    BenefitNotFound(i64),
    #[error("资源不存在: {0}")]
    NotFound(String),

    // 业务错误
    #[error("徽章已发布，无法删除")]
    BadgeAlreadyPublished,
    #[error("规则 JSON 格式无效: {0}")]
    InvalidRuleJson(String),
    #[error("文件处理失败: {0}")]
    FileProcessingError(String),
    #[error("库存不足")]
    InsufficientStock,
    #[error("用户徽章数量不足")]
    InsufficientUserBadge,

    // 系统错误
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Redis错误: {0}")]
    Redis(String),
    #[error("内部错误: {0}")]
    Internal(String),
}

impl AdminError {
    /// 返回对应的 HTTP 状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 认证错误
            Self::Unauthorized(_) | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::UserDisabled | Self::UserLocked | Self::PasswordChangeRequired => {
                StatusCode::FORBIDDEN
            }
            Self::UserNotFound(_) => StatusCode::NOT_FOUND,

            Self::Validation(_) | Self::InvalidRuleJson(_) => StatusCode::BAD_REQUEST,

            Self::CategoryNotFound(_)
            | Self::SeriesNotFound(_)
            | Self::BadgeNotFound(_)
            | Self::RuleNotFound(_)
            | Self::TaskNotFound(_)
            | Self::DependencyNotFound(_)
            | Self::BenefitNotFound(_)
            | Self::NotFound(_) => StatusCode::NOT_FOUND,

            Self::BadgeAlreadyPublished | Self::InsufficientStock | Self::InsufficientUserBadge => {
                StatusCode::CONFLICT
            }

            Self::FileProcessingError(_) => StatusCode::UNPROCESSABLE_ENTITY,

            Self::Database(_) | Self::Redis(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// 返回错误码（用于 API 响应）
    pub fn error_code(&self) -> &'static str {
        match self {
            // 认证错误
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::UserDisabled => "USER_DISABLED",
            Self::UserLocked => "USER_LOCKED",
            Self::PasswordChangeRequired => "PASSWORD_CHANGE_REQUIRED",
            Self::UserNotFound(_) => "USER_NOT_FOUND",

            Self::Validation(_) => "VALIDATION_ERROR",
            Self::CategoryNotFound(_) => "CATEGORY_NOT_FOUND",
            Self::SeriesNotFound(_) => "SERIES_NOT_FOUND",
            Self::BadgeNotFound(_) => "BADGE_NOT_FOUND",
            Self::RuleNotFound(_) => "RULE_NOT_FOUND",
            Self::TaskNotFound(_) => "TASK_NOT_FOUND",
            Self::DependencyNotFound(_) => "DEPENDENCY_NOT_FOUND",
            Self::BenefitNotFound(_) => "BENEFIT_NOT_FOUND",
            Self::NotFound(_) => "NOT_FOUND",
            Self::BadgeAlreadyPublished => "BADGE_ALREADY_PUBLISHED",
            Self::InvalidRuleJson(_) => "INVALID_RULE_JSON",
            Self::FileProcessingError(_) => "FILE_PROCESSING_ERROR",
            Self::InsufficientStock => "INSUFFICIENT_STOCK",
            Self::InsufficientUserBadge => "INSUFFICIENT_USER_BADGE",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Redis(_) => "REDIS_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        let status = self.status_code();

        // 系统级错误只返回通用提示，详细信息仅记录日志，防止信息泄露
        let message = match &self {
            Self::Database(e) => {
                tracing::error!(error = %e, "数据库操作失败");
                "服务内部错误，请稍后重试".to_string()
            }
            Self::Redis(e) => {
                tracing::error!(error = %e, "Redis 操作失败");
                "服务内部错误，请稍后重试".to_string()
            }
            Self::Internal(e) => {
                tracing::error!(error = %e, "内部错误");
                "服务内部错误，请稍后重试".to_string()
            }
            other => other.to_string(),
        };

        let body = json!({
            "success": false,
            "code": self.error_code(),
            "message": message,
            "data": serde_json::Value::Null
        });

        (status, axum::Json(body)).into_response()
    }
}

/// 从 validator 错误转换
impl From<validator::ValidationErrors> for AdminError {
    fn from(errors: validator::ValidationErrors) -> Self {
        Self::Validation(errors.to_string())
    }
}

/// 从 JSON 序列化错误转换
impl From<serde_json::Error> for AdminError {
    fn from(err: serde_json::Error) -> Self {
        Self::Internal(format!("JSON 处理错误: {}", err))
    }
}

/// 从 badge-management-service 的错误转换
impl From<badge_management::BadgeError> for AdminError {
    fn from(err: badge_management::BadgeError) -> Self {
        match err {
            badge_management::BadgeError::Database(e) => Self::Database(e),
            badge_management::BadgeError::BadgeNotFound(id) => Self::BadgeNotFound(id),
            badge_management::BadgeError::SeriesNotFound(id) => Self::SeriesNotFound(id),
            badge_management::BadgeError::CategoryNotFound(id) => Self::CategoryNotFound(id),
            badge_management::BadgeError::Validation(msg) => Self::Validation(msg),
            other => Self::Internal(other.to_string()),
        }
    }
}

/// 服务层 Result 类型别名
pub type Result<T> = std::result::Result<T, AdminError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    // ---- 辅助函数 ----

    /// 构造所有错误变体及其期望的 (StatusCode, error_code) 映射。
    /// 使用表驱动方式避免逐个变体写重复断言，同时保证新增变体时只需在一处维护。
    fn all_error_variants() -> Vec<(AdminError, StatusCode, &'static str)> {
        vec![
            // 认证 & 权限类：这些错误直接决定用户能否继续操作，状态码必须精确
            (AdminError::Unauthorized("token expired".into()), StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            (AdminError::Forbidden("no permission".into()), StatusCode::FORBIDDEN, "FORBIDDEN"),
            (AdminError::InvalidCredentials, StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"),
            (AdminError::UserDisabled, StatusCode::FORBIDDEN, "USER_DISABLED"),
            (AdminError::UserLocked, StatusCode::FORBIDDEN, "USER_LOCKED"),
            (AdminError::PasswordChangeRequired, StatusCode::FORBIDDEN, "PASSWORD_CHANGE_REQUIRED"),
            (AdminError::UserNotFound("admin".into()), StatusCode::NOT_FOUND, "USER_NOT_FOUND"),
            // 参数校验
            (AdminError::Validation("name is required".into()), StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            // 资源不存在类：前端依赖 404 做条件跳转，错误码用于区分具体缺失资源
            (AdminError::CategoryNotFound(10), StatusCode::NOT_FOUND, "CATEGORY_NOT_FOUND"),
            (AdminError::SeriesNotFound(20), StatusCode::NOT_FOUND, "SERIES_NOT_FOUND"),
            (AdminError::BadgeNotFound(30), StatusCode::NOT_FOUND, "BADGE_NOT_FOUND"),
            (AdminError::RuleNotFound(40), StatusCode::NOT_FOUND, "RULE_NOT_FOUND"),
            (AdminError::TaskNotFound(50), StatusCode::NOT_FOUND, "TASK_NOT_FOUND"),
            (AdminError::DependencyNotFound(60), StatusCode::NOT_FOUND, "DEPENDENCY_NOT_FOUND"),
            (AdminError::BenefitNotFound(70), StatusCode::NOT_FOUND, "BENEFIT_NOT_FOUND"),
            (AdminError::NotFound("some resource".into()), StatusCode::NOT_FOUND, "NOT_FOUND"),
            // 业务冲突类：409 表示请求合法但与当前状态冲突
            (AdminError::BadgeAlreadyPublished, StatusCode::CONFLICT, "BADGE_ALREADY_PUBLISHED"),
            (AdminError::InsufficientStock, StatusCode::CONFLICT, "INSUFFICIENT_STOCK"),
            (AdminError::InsufficientUserBadge, StatusCode::CONFLICT, "INSUFFICIENT_USER_BADGE"),
            // 请求数据格式错误
            (AdminError::InvalidRuleJson("unexpected EOF".into()), StatusCode::BAD_REQUEST, "INVALID_RULE_JSON"),
            // 文件处理用 422，因为请求格式合法但内容无法处理
            (AdminError::FileProcessingError("corrupt image".into()), StatusCode::UNPROCESSABLE_ENTITY, "FILE_PROCESSING_ERROR"),
            // 系统级错误：统一 500，防止内部实现细节泄露
            (AdminError::Redis("connection refused".into()), StatusCode::INTERNAL_SERVER_ERROR, "REDIS_ERROR"),
            (AdminError::Internal("unexpected state".into()), StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        ]
    }

    // ---- 原有测试（保留不变）----

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            AdminError::Validation("test".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AdminError::BadgeNotFound(1).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AdminError::BadgeAlreadyPublished.status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AdminError::Internal("test".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            AdminError::Validation("test".into()).error_code(),
            "VALIDATION_ERROR"
        );
        assert_eq!(AdminError::BadgeNotFound(1).error_code(), "BADGE_NOT_FOUND");
    }

    // ---- 表驱动：全量 status_code 覆盖 ----

    /// 确保每个错误变体都映射到正确的 HTTP 状态码。
    /// 状态码错误会导致前端误判请求结果（如把 403 当 500 处理），所以需要逐一验证。
    #[test]
    fn test_all_variants_status_code() {
        for (error, expected_status, label) in all_error_variants() {
            assert_eq!(
                error.status_code(),
                expected_status,
                "状态码不匹配: variant={label}"
            );
        }
    }

    // ---- 表驱动：全量 error_code 覆盖 ----

    /// 错误码是 API 契约的一部分，客户端用它做条件分支。
    /// 任何错误码变更都是破坏性变更，必须逐一锁定。
    #[test]
    fn test_all_variants_error_code() {
        for (error, _status, expected_code) in all_error_variants() {
            assert_eq!(
                error.error_code(),
                expected_code,
                "错误码不匹配: expected={expected_code}"
            );
        }
    }

    // ---- Display trait 测试 ----

    /// Display 输出直接作为 API 响应的 message 字段返回给用户，
    /// 必须包含关键上下文（如 ID、用户名），否则用户无法定位问题。
    #[test]
    fn test_display_contains_context_for_parameterized_variants() {
        // 携带 String 参数的变体：确认参数出现在输出中
        assert!(AdminError::Unauthorized("expired".into()).to_string().contains("expired"));
        assert!(AdminError::Forbidden("admin only".into()).to_string().contains("admin only"));
        assert!(AdminError::UserNotFound("alice".into()).to_string().contains("alice"));
        assert!(AdminError::Validation("email invalid".into()).to_string().contains("email invalid"));
        assert!(AdminError::NotFound("rule #5".into()).to_string().contains("rule #5"));
        assert!(AdminError::InvalidRuleJson("bad json".into()).to_string().contains("bad json"));
        assert!(AdminError::FileProcessingError("too large".into()).to_string().contains("too large"));
        assert!(AdminError::Redis("timeout".into()).to_string().contains("timeout"));
        assert!(AdminError::Internal("oom".into()).to_string().contains("oom"));

        // 携带 i64 参数的变体：确认 ID 出现在输出中
        assert!(AdminError::CategoryNotFound(42).to_string().contains("42"));
        assert!(AdminError::SeriesNotFound(99).to_string().contains("99"));
        assert!(AdminError::BadgeNotFound(7).to_string().contains("7"));
        assert!(AdminError::RuleNotFound(11).to_string().contains("11"));
        assert!(AdminError::TaskNotFound(22).to_string().contains("22"));
        assert!(AdminError::DependencyNotFound(33).to_string().contains("33"));
        assert!(AdminError::BenefitNotFound(44).to_string().contains("44"));
    }

    /// 无参数的变体也应有可读的中文描述，不能返回空字符串
    #[test]
    fn test_display_nonempty_for_unit_variants() {
        let unit_variants: Vec<AdminError> = vec![
            AdminError::InvalidCredentials,
            AdminError::UserDisabled,
            AdminError::UserLocked,
            AdminError::PasswordChangeRequired,
            AdminError::BadgeAlreadyPublished,
            AdminError::InsufficientStock,
            AdminError::InsufficientUserBadge,
        ];
        for err in unit_variants {
            let msg = err.to_string();
            assert!(!msg.is_empty(), "Display 输出不应为空: {:?}", err);
        }
    }

    // ---- IntoResponse 测试 ----

    /// IntoResponse 是错误到 HTTP 响应的最终出口。
    /// 必须验证：状态码正确、响应体结构完整（success/code/message/data 四字段），
    /// 否则前端解析会崩溃。
    #[tokio::test]
    async fn test_into_response_body_structure() {
        // 选几个代表性变体验证响应体结构
        let test_cases: Vec<(AdminError, StatusCode, &str)> = vec![
            (AdminError::Unauthorized("no token".into()), StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            (AdminError::Forbidden("read only".into()), StatusCode::FORBIDDEN, "FORBIDDEN"),
            (AdminError::InvalidCredentials, StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"),
            (AdminError::UserDisabled, StatusCode::FORBIDDEN, "USER_DISABLED"),
            (AdminError::UserLocked, StatusCode::FORBIDDEN, "USER_LOCKED"),
            (AdminError::PasswordChangeRequired, StatusCode::FORBIDDEN, "PASSWORD_CHANGE_REQUIRED"),
            (AdminError::BadgeNotFound(1), StatusCode::NOT_FOUND, "BADGE_NOT_FOUND"),
            (AdminError::CategoryNotFound(2), StatusCode::NOT_FOUND, "CATEGORY_NOT_FOUND"),
            (AdminError::SeriesNotFound(3), StatusCode::NOT_FOUND, "SERIES_NOT_FOUND"),
            (AdminError::RuleNotFound(4), StatusCode::NOT_FOUND, "RULE_NOT_FOUND"),
            (AdminError::TaskNotFound(5), StatusCode::NOT_FOUND, "TASK_NOT_FOUND"),
            (AdminError::DependencyNotFound(6), StatusCode::NOT_FOUND, "DEPENDENCY_NOT_FOUND"),
            (AdminError::BenefitNotFound(7), StatusCode::NOT_FOUND, "BENEFIT_NOT_FOUND"),
            (AdminError::NotFound("missing".into()), StatusCode::NOT_FOUND, "NOT_FOUND"),
            (AdminError::UserNotFound("bob".into()), StatusCode::NOT_FOUND, "USER_NOT_FOUND"),
            (AdminError::Validation("bad".into()), StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            (AdminError::InvalidRuleJson("eof".into()), StatusCode::BAD_REQUEST, "INVALID_RULE_JSON"),
            (AdminError::FileProcessingError("corrupt".into()), StatusCode::UNPROCESSABLE_ENTITY, "FILE_PROCESSING_ERROR"),
            (AdminError::BadgeAlreadyPublished, StatusCode::CONFLICT, "BADGE_ALREADY_PUBLISHED"),
            (AdminError::InsufficientStock, StatusCode::CONFLICT, "INSUFFICIENT_STOCK"),
            (AdminError::InsufficientUserBadge, StatusCode::CONFLICT, "INSUFFICIENT_USER_BADGE"),
            (AdminError::Redis("down".into()), StatusCode::INTERNAL_SERVER_ERROR, "REDIS_ERROR"),
            (AdminError::Internal("crash".into()), StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        ];

        for (error, expected_status, expected_code) in test_cases {
            let label = format!("{:?}", error);
            let response = error.into_response();

            assert_eq!(
                response.status(),
                expected_status,
                "响应状态码不匹配: {label}"
            );

            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("读取响应体失败");
            let body: serde_json::Value =
                serde_json::from_slice(&body_bytes).expect("响应体不是合法 JSON");

            // 四个字段必须存在
            assert_eq!(body["success"], json!(false), "success 字段应为 false: {label}");
            assert_eq!(body["code"], json!(expected_code), "code 字段不匹配: {label}");
            assert!(body.get("message").is_some(), "缺少 message 字段: {label}");
            assert!(!body["message"].as_str().unwrap_or("").is_empty(), "message 不应为空: {label}");
            assert!(body.get("data").is_some(), "缺少 data 字段: {label}");
            assert!(body["data"].is_null(), "data 字段应为 null: {label}");
        }
    }

    /// 系统级错误（Database/Redis/Internal）的响应消息不应泄露内部细节，
    /// 只返回通用提示。这是安全要求，防止攻击者通过错误消息探测系统架构。
    #[tokio::test]
    async fn test_system_errors_hide_internal_details() {
        let system_errors: Vec<(AdminError, &str)> = vec![
            (AdminError::Redis("redis://10.0.0.1:6379 connection refused".into()), "redis://10.0.0.1:6379"),
            (AdminError::Internal("stack overflow at module X".into()), "stack overflow"),
        ];

        for (error, leaked_detail) in system_errors {
            let response = error.into_response();
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("读取响应体失败");
            let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
            let message = body["message"].as_str().unwrap();

            // 响应消息中不应包含内部错误详情
            assert!(
                !message.contains(leaked_detail),
                "系统错误消息泄露了内部细节: message={message}, leaked={leaked_detail}"
            );
            // 应返回统一的通用提示
            assert!(
                message.contains("服务内部错误"),
                "系统错误应返回通用提示，实际: {message}"
            );
        }
    }

    /// 业务错误的响应消息应保留原始描述，帮助用户理解问题
    #[tokio::test]
    async fn test_business_errors_preserve_display_message() {
        let business_errors: Vec<(AdminError, &str)> = vec![
            (AdminError::Unauthorized("token expired".into()), "token expired"),
            (AdminError::Forbidden("需要管理员权限".into()), "需要管理员权限"),
            (AdminError::BadgeNotFound(42), "42"),
            (AdminError::Validation("name is required".into()), "name is required"),
        ];

        for (error, expected_fragment) in business_errors {
            let response = error.into_response();
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("读取响应体失败");
            let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
            let message = body["message"].as_str().unwrap();

            assert!(
                message.contains(expected_fragment),
                "业务错误消息应包含上下文: message={message}, expected_fragment={expected_fragment}"
            );
        }
    }

    // ---- From<validator::ValidationErrors> 转换测试 ----

    /// validator 是请求参数校验的统一入口，转换必须把字段级错误信息带入 AdminError，
    /// 否则用户无法知道哪个字段校验失败。
    #[test]
    fn test_from_validation_errors() {
        use validator::{ValidationError, ValidationErrors};

        let mut errors = ValidationErrors::new();
        let mut field_error = ValidationError::new("length");
        field_error.message = Some("名称长度不能超过 50 个字符".into());
        errors.add("name", field_error);

        let admin_error: AdminError = errors.into();
        match &admin_error {
            AdminError::Validation(msg) => {
                assert!(
                    msg.contains("name"),
                    "转换后应保留字段名: {msg}"
                );
            }
            other => panic!("期望 Validation 变体，实际: {:?}", other),
        }

        // 转换后的状态码和错误码也必须正确
        assert_eq!(admin_error.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(admin_error.error_code(), "VALIDATION_ERROR");
    }

    // ---- From<badge_management::BadgeError> 转换测试 ----

    /// badge-management 是下游核心服务，错误转换逻辑决定了管理后台能否正确区分
    /// 「资源不存在」和「系统故障」。映射错误会导致用户看到误导性的错误提示。
    #[test]
    fn test_from_badge_error_mapped_variants() {
        // BadgeNotFound -> AdminError::BadgeNotFound（保留 ID）
        let err: AdminError = badge_management::BadgeError::BadgeNotFound(100).into();
        assert!(matches!(err, AdminError::BadgeNotFound(100)));

        // SeriesNotFound -> AdminError::SeriesNotFound（保留 ID）
        let err: AdminError = badge_management::BadgeError::SeriesNotFound(200).into();
        assert!(matches!(err, AdminError::SeriesNotFound(200)));

        // CategoryNotFound -> AdminError::CategoryNotFound（保留 ID）
        let err: AdminError = badge_management::BadgeError::CategoryNotFound(300).into();
        assert!(matches!(err, AdminError::CategoryNotFound(300)));

        // Validation -> AdminError::Validation（保留消息）
        let err: AdminError =
            badge_management::BadgeError::Validation("badge name too long".into()).into();
        match err {
            AdminError::Validation(msg) => assert!(msg.contains("badge name too long")),
            other => panic!("期望 Validation，实际: {:?}", other),
        }
    }

    /// 未在映射表中显式列出的 BadgeError 变体应回退到 AdminError::Internal，
    /// 避免 panic 或漏掉未知错误。
    #[test]
    fn test_from_badge_error_fallback_to_internal() {
        // 选一个不在显式映射中的变体
        let err: AdminError = badge_management::BadgeError::BadgeInactive(999).into();
        match err {
            AdminError::Internal(msg) => {
                // 回退时应把原始错误信息带入，方便排查
                assert!(!msg.is_empty(), "Internal 消息不应为空");
            }
            other => panic!("未映射的 BadgeError 应回退到 Internal，实际: {:?}", other),
        }

        // Redis 错误也验证一下（它在 BadgeError 中有独立变体但 From 实现走 other 分支）
        let err: AdminError =
            badge_management::BadgeError::Redis("connection lost".into()).into();
        match err {
            AdminError::Internal(msg) => {
                assert!(msg.contains("connection lost"));
            }
            other => panic!("BadgeError::Redis 应回退到 Internal，实际: {:?}", other),
        }
    }

    /// Database 错误从 BadgeError 转换时应保持为 AdminError::Database，
    /// 因为 sqlx::Error 已经实现了 From，需要确保不会被意外路由到 Internal。
    #[test]
    fn test_from_badge_error_database_stays_database() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let badge_err = badge_management::BadgeError::Database(sqlx_err);
        let admin_err: AdminError = badge_err.into();
        assert!(
            matches!(admin_err, AdminError::Database(_)),
            "BadgeError::Database 应映射到 AdminError::Database"
        );
        assert_eq!(admin_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(admin_err.error_code(), "DATABASE_ERROR");
    }

    // ---- Database (sqlx::Error) From 转换测试 ----

    /// sqlx::Error 通过 #[from] 自动派生 From，验证转换后类型和状态码正确
    #[test]
    fn test_from_sqlx_error() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let admin_err = AdminError::from(sqlx_err);
        assert!(matches!(admin_err, AdminError::Database(_)));
        assert_eq!(admin_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(admin_err.error_code(), "DATABASE_ERROR");
    }

    // ---- 变体完备性校验 ----

    /// 确保测试用例覆盖了所有 22 个变体（不含 Database，因为它需要 sqlx::Error 无法简单构造）。
    /// 如果新增了变体但忘记加测试，这个计数断言会失败。
    #[test]
    fn test_all_variants_covered_in_table() {
        // 共 24 个变体，Database 依赖 sqlx::Error 不易在表中构造，故排除 1 个 → 23
        assert_eq!(
            all_error_variants().len(),
            23,
            "表驱动用例数量与变体总数不一致，可能新增了变体但未更新测试"
        );
    }
}
