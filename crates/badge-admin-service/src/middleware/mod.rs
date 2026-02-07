//! 中间件模块
//!
//! 提供认证、权限检查和审计日志中间件

pub mod audit;
mod auth;
mod api_key_auth;
mod permission;

pub use audit::audit_middleware;
pub use auth::auth_middleware;
pub use api_key_auth::{api_key_auth_middleware, ApiKeyContext, ExternalApiState, extract_api_key_context, check_api_key_permission, require_api_key_permission};
pub use permission::require_permission;
