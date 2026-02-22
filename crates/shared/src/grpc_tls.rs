//! gRPC TLS 配置辅助模块
//!
//! 为 tonic 的 gRPC 服务端和客户端提供统一的 TLS 配置加载，
//! 根据 `TlsConfig.enabled` 决定是否启用加密通信。
//! 开发环境默认不启用，生产环境通过配置文件或环境变量指定证书路径。

use std::path::Path;

use tonic::transport::{Certificate, ClientTlsConfig, Identity, ServerTlsConfig};
use tracing::info;

use crate::config::TlsConfig;

/// TLS 配置加载错误
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("TLS 已启用但未配置证书路径 (cert_path)")]
    MissingCertPath,

    #[error("TLS 已启用但未配置私钥路径 (key_path)")]
    MissingKeyPath,

    #[error("读取证书文件失败: {path}: {source}")]
    ReadCert {
        path: String,
        source: std::io::Error,
    },

    #[error("读取私钥文件失败: {path}: {source}")]
    ReadKey {
        path: String,
        source: std::io::Error,
    },

    #[error("读取 CA 证书失败: {path}: {source}")]
    ReadCa {
        path: String,
        source: std::io::Error,
    },
}

/// 从配置构建 gRPC 服务端 TLS 配置
///
/// 返回 None 表示 TLS 未启用（开发环境），
/// 返回 Some 表示已加载证书可用于 `Server::builder().tls_config()`。
pub async fn build_server_tls_config(
    tls_config: &TlsConfig,
) -> Result<Option<ServerTlsConfig>, TlsError> {
    if !tls_config.enabled {
        return Ok(None);
    }

    let cert_path = tls_config
        .cert_path
        .as_deref()
        .ok_or(TlsError::MissingCertPath)?;
    let key_path = tls_config
        .key_path
        .as_deref()
        .ok_or(TlsError::MissingKeyPath)?;

    let cert = read_file(cert_path)
        .await
        .map_err(|e| TlsError::ReadCert {
            path: cert_path.to_string(),
            source: e,
        })?;
    let key = read_file(key_path).await.map_err(|e| TlsError::ReadKey {
        path: key_path.to_string(),
        source: e,
    })?;

    let identity = Identity::from_pem(cert, key);
    let mut config = ServerTlsConfig::new().identity(identity);

    // 如果配置了 CA 证书，启用双向 TLS（mTLS）验证客户端
    if let Some(ref ca_path) = tls_config.ca_path {
        let ca_cert = read_file(ca_path).await.map_err(|e| TlsError::ReadCa {
            path: ca_path.to_string(),
            source: e,
        })?;
        config = config.client_ca_root(Certificate::from_pem(ca_cert));
    }

    info!(cert_path, key_path, "gRPC 服务端 TLS 已配置");
    Ok(Some(config))
}

/// 从配置构建 gRPC 客户端 TLS 配置
///
/// 返回 None 表示 TLS 未启用，客户端使用 `http://` 明文连接；
/// 返回 Some 时客户端需使用 `https://` 并附加此 TLS 配置。
pub async fn build_client_tls_config(
    tls_config: &TlsConfig,
) -> Result<Option<ClientTlsConfig>, TlsError> {
    if !tls_config.enabled {
        return Ok(None);
    }

    let mut config = ClientTlsConfig::new();

    // CA 证书用于验证服务端身份
    if let Some(ref ca_path) = tls_config.ca_path {
        let ca_cert = read_file(ca_path).await.map_err(|e| TlsError::ReadCa {
            path: ca_path.to_string(),
            source: e,
        })?;
        config = config.ca_certificate(Certificate::from_pem(ca_cert));
    }

    // 如果同时配置了客户端证书和私钥，启用 mTLS
    if let (Some(cert_path), Some(key_path)) =
        (&tls_config.cert_path, &tls_config.key_path)
    {
        let cert = read_file(cert_path)
            .await
            .map_err(|e| TlsError::ReadCert {
                path: cert_path.to_string(),
                source: e,
            })?;
        let key = read_file(key_path).await.map_err(|e| TlsError::ReadKey {
            path: key_path.to_string(),
            source: e,
        })?;
        config = config.identity(Identity::from_pem(cert, key));
    }

    info!("gRPC 客户端 TLS 已配置");
    Ok(Some(config))
}

/// 根据 TLS 配置决定 gRPC 连接 URI 的 scheme
///
/// TLS 启用时返回 `https://`，否则返回 `http://`。
/// 用于构建 gRPC 客户端连接地址。
pub fn grpc_scheme(tls_config: &TlsConfig) -> &'static str {
    if tls_config.enabled {
        "https"
    } else {
        "http"
    }
}

async fn read_file(path: &str) -> Result<Vec<u8>, std::io::Error> {
    // 在读取前验证路径存在，提供更明确的错误信息
    if !Path::new(path).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("文件不存在: {path}"),
        ));
    }
    tokio::fs::read(path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_scheme_disabled() {
        let config = TlsConfig::default();
        assert_eq!(grpc_scheme(&config), "http");
    }

    #[test]
    fn test_grpc_scheme_enabled() {
        let config = TlsConfig {
            enabled: true,
            cert_path: Some("/tmp/cert.pem".into()),
            key_path: Some("/tmp/key.pem".into()),
            ca_path: None,
        };
        assert_eq!(grpc_scheme(&config), "https");
    }

    #[tokio::test]
    async fn test_server_tls_disabled() {
        let config = TlsConfig::default();
        let result = build_server_tls_config(&config).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_server_tls_missing_cert() {
        let config = TlsConfig {
            enabled: true,
            cert_path: None,
            key_path: Some("/tmp/key.pem".into()),
            ca_path: None,
        };
        let result = build_server_tls_config(&config).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::MissingCertPath));
    }

    #[tokio::test]
    async fn test_server_tls_missing_key() {
        let config = TlsConfig {
            enabled: true,
            cert_path: Some("/tmp/cert.pem".into()),
            key_path: None,
            ca_path: None,
        };
        let result = build_server_tls_config(&config).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::MissingKeyPath));
    }

    #[tokio::test]
    async fn test_client_tls_disabled() {
        let config = TlsConfig::default();
        let result = build_client_tls_config(&config).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_tls_error_display() {
        let err = TlsError::MissingCertPath;
        assert!(err.to_string().contains("cert_path"));

        let err = TlsError::MissingKeyPath;
        assert!(err.to_string().contains("key_path"));
    }
}
