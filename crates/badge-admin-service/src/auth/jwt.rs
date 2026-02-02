//! JWT Token 处理
//!
//! 提供 JWT Token 的生成和验证功能

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AdminError;

/// JWT 配置
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// 签名密钥
    pub secret: String,
    /// Token 过期时间（秒）
    pub expires_in_secs: i64,
    /// Token 签发者
    pub issuer: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "badge-admin-secret-key-change-in-production".to_string(),
            expires_in_secs: 86400, // 24 小时
            issuer: "badge-admin-service".to_string(),
        }
    }
}

/// JWT Claims（Token 载荷）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// 用户 ID
    pub sub: String,
    /// 用户名
    pub username: String,
    /// 显示名称
    pub display_name: Option<String>,
    /// 角色列表
    pub roles: Vec<String>,
    /// 权限列表
    pub permissions: Vec<String>,
    /// 签发时间
    pub iat: i64,
    /// 过期时间
    pub exp: i64,
    /// 签发者
    pub iss: String,
}

/// JWT 管理器
#[derive(Clone)]
pub struct JwtManager {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    /// 创建 JWT 管理器
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// 生成 JWT Token
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `username`: 用户名
    /// - `display_name`: 显示名称
    /// - `roles`: 角色列表
    /// - `permissions`: 权限列表
    pub fn generate_token(
        &self,
        user_id: i64,
        username: &str,
        display_name: Option<&str>,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Result<(String, i64), AdminError> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.expires_in_secs);

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            display_name: display_name.map(|s| s.to_string()),
            roles,
            permissions,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: self.config.issuer.clone(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AdminError::Internal(format!("JWT 生成失败: {}", e)))?;

        Ok((token, exp.timestamp()))
    }

    /// 验证并解析 JWT Token
    ///
    /// 返回解析后的 Claims，如果 Token 无效或过期则返回错误
    pub fn verify_token(&self, token: &str) -> Result<Claims, AdminError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation).map_err(
            |e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    AdminError::Unauthorized("Token 已过期".to_string())
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken => {
                    AdminError::Unauthorized("无效的 Token".to_string())
                }
                _ => AdminError::Unauthorized(format!("Token 验证失败: {}", e)),
            },
        )?;

        Ok(token_data.claims)
    }

    /// 刷新 Token
    ///
    /// 基于现有的 Claims 生成新的 Token（延长过期时间）
    pub fn refresh_token(&self, claims: &Claims) -> Result<(String, i64), AdminError> {
        let user_id: i64 = claims
            .sub
            .parse()
            .map_err(|_| AdminError::Internal("无效的用户 ID".to_string()))?;

        self.generate_token(
            user_id,
            &claims.username,
            claims.display_name.as_deref(),
            claims.roles.clone(),
            claims.permissions.clone(),
        )
    }

    /// 获取 Token 过期时间（秒）
    pub fn expires_in_secs(&self) -> i64 {
        self.config.expires_in_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify_token() {
        let config = JwtConfig::default();
        let manager = JwtManager::new(config);

        let (token, _exp) = manager
            .generate_token(
                1,
                "admin",
                Some("管理员"),
                vec!["admin".to_string()],
                vec!["system:user:read".to_string()],
            )
            .unwrap();

        let claims = manager.verify_token(&token).unwrap();
        assert_eq!(claims.sub, "1");
        assert_eq!(claims.username, "admin");
        assert_eq!(claims.roles, vec!["admin"]);
    }

    #[test]
    fn test_invalid_token() {
        let config = JwtConfig::default();
        let manager = JwtManager::new(config);

        let result = manager.verify_token("invalid.token.here");
        assert!(result.is_err());
    }
}
