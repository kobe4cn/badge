//! 配置管理模块
//!
//! 支持多格式配置文件加载，环境变量覆盖，以及类型安全的配置访问。

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::Path;

/// 数据库配置
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://badge:badge_secret@localhost:5432/badge_db".to_string(),
            max_connections: 10,
            min_connections: 2,
            connect_timeout_seconds: 30,
            idle_timeout_seconds: 600,
        }
    }
}

/// Redis 配置
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
        }
    }
}

/// Kafka 配置
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    pub consumer_group: String,
    pub auto_offset_reset: String,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: "localhost:9092".to_string(),
            consumer_group: "badge-service".to_string(),
            auto_offset_reset: "earliest".to_string(),
        }
    }
}

/// 服务配置
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: None,
        }
    }
}

/// 可观测性配置
#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    pub log_level: String,
    /// 日志输出格式：json（结构化）或 pretty（人类可读）
    pub log_format: String,
    pub metrics_enabled: bool,
    pub metrics_port: u16,
    pub tracing_enabled: bool,
    pub tracing_endpoint: Option<String>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_format: "pretty".to_string(),
            metrics_enabled: true,
            metrics_port: 9090,
            tracing_enabled: false,
            tracing_endpoint: None,
        }
    }
}

/// 应用配置
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppConfig {
    pub service_name: String,
    pub environment: String,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub kafka: KafkaConfig,
    pub observability: ObservabilityConfig,
}

impl AppConfig {
    /// 从配置文件和环境变量加载配置
    ///
    /// 加载顺序（后加载的会覆盖先加载的同名配置项）：
    /// 1. config/default.toml（默认配置）
    /// 2. config/{environment}.toml（环境特定配置）
    /// 3. 环境变量（BADGE_ 前缀，如 BADGE_DATABASE_URL -> database.url）
    pub fn load(service_name: &str) -> Result<Self, ConfigError> {
        let env = std::env::var("BADGE_ENV").unwrap_or_else(|_| "development".to_string());

        let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| "config".to_string());

        let builder = Config::builder()
            // 默认配置
            .set_default("service_name", service_name)?
            .set_default("environment", env.clone())?
            // 加载默认配置文件
            .add_source(File::from(Path::new(&config_dir).join("default.toml")).required(false))
            // 加载环境特定配置
            .add_source(
                File::from(Path::new(&config_dir).join(format!("{}.toml", env))).required(false),
            )
            // 环境变量覆盖（BADGE_DATABASE_URL -> database.url）
            .add_source(
                Environment::with_prefix("BADGE")
                    .separator("_")
                    .try_parsing(true),
            );

        builder.build()?.try_deserialize()
    }

    /// 获取服务地址
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// 是否为生产环境
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.max_connections, 10);
    }

    #[test]
    fn test_server_addr() {
        let config = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                workers: None,
            },
            ..Default::default()
        };
        assert_eq!(config.server_addr(), "127.0.0.1:3000");
    }
}
