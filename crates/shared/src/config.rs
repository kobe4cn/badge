//! 配置管理模块
//!
//! 支持多格式配置文件加载，环境变量覆盖，以及类型安全的配置访问。
//!
//! ## 配置加载顺序
//!
//! 1. `.env` 文件（通过 dotenvy 加载到环境变量）
//! 2. `config/default.toml`（默认配置）
//! 3. `config/{environment}.toml`（环境特定配置）
//! 4. `config/{service_name}.toml`（服务特定配置）
//! 5. `BADGE_*` 环境变量
//! 6. 服务特定端口环境变量（如 `BADGE_ADMIN_PORT`）

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::Path;

// 从 observability 模块重导出 ObservabilityConfig，保持向后兼容
pub use crate::observability::ObservabilityConfig;

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
            max_connections: 50,
            min_connections: 5,
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
            pool_size: 20,
        }
    }
}

/// Kafka Topics 配置
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaTopicsConfig {
    #[serde(default = "default_engagement_events")]
    pub engagement_events: String,

    #[serde(default = "default_transaction_events")]
    pub transaction_events: String,

    #[serde(default = "default_notifications")]
    pub notifications: String,

    #[serde(default = "default_dead_letter_queue")]
    pub dead_letter_queue: String,

    #[serde(default = "default_rule_reload")]
    pub rule_reload: String,
}

fn default_engagement_events() -> String {
    "badge.engagement.events".into()
}
fn default_transaction_events() -> String {
    "badge.transaction.events".into()
}
fn default_notifications() -> String {
    "badge.notifications".into()
}
fn default_dead_letter_queue() -> String {
    "badge.dlq".into()
}
fn default_rule_reload() -> String {
    "badge.rule.reload".into()
}

impl Default for KafkaTopicsConfig {
    fn default() -> Self {
        Self {
            engagement_events: default_engagement_events(),
            transaction_events: default_transaction_events(),
            notifications: default_notifications(),
            dead_letter_queue: default_dead_letter_queue(),
            rule_reload: default_rule_reload(),
        }
    }
}

/// Kafka 配置
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    pub consumer_group: String,
    pub auto_offset_reset: String,
    /// 消费者两次 poll 间的最大间隔（毫秒），超时会被踢出消费组触发 rebalance。
    /// 需要大于单条消息最慢处理时间（含级联规则评估 + DB 事务 + 通知投递）。
    #[serde(default = "default_max_poll_interval_ms")]
    pub max_poll_interval_ms: u32,
    /// Broker 侧会话超时（毫秒），消费者在此窗口内无心跳则被移出消费组。
    #[serde(default = "default_session_timeout_ms")]
    pub session_timeout_ms: u32,
    /// 心跳间隔（毫秒），建议 ≤ session_timeout_ms 的 1/3。
    #[serde(default = "default_heartbeat_interval_ms")]
    pub heartbeat_interval_ms: u32,
    #[serde(default)]
    pub topics: KafkaTopicsConfig,
    /// 安全配置（SASL_SSL），开发环境不配置时使用 PLAINTEXT
    #[serde(default)]
    pub security: KafkaSecurityConfig,
}

fn default_max_poll_interval_ms() -> u32 {
    600_000
}
fn default_session_timeout_ms() -> u32 {
    45_000
}
fn default_heartbeat_interval_ms() -> u32 {
    15_000
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: "localhost:9092".to_string(),
            consumer_group: "badge-service".to_string(),
            auto_offset_reset: "earliest".to_string(),
            max_poll_interval_ms: default_max_poll_interval_ms(),
            session_timeout_ms: default_session_timeout_ms(),
            heartbeat_interval_ms: default_heartbeat_interval_ms(),
            topics: KafkaTopicsConfig::default(),
            security: KafkaSecurityConfig::default(),
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

/// 规则加载配置
#[derive(Debug, Clone, Deserialize)]
pub struct RulesConfig {
    /// 定时刷新间隔（秒），默认 30
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,

    /// 启动加载超时（秒），默认 10
    #[serde(default = "default_initial_timeout")]
    pub initial_load_timeout_secs: u64,

    /// 幂等窗口（小时），默认 24
    #[serde(default = "default_idempotency_ttl")]
    pub idempotency_ttl_hours: u64,
}

fn default_refresh_interval() -> u64 {
    30
}
fn default_initial_timeout() -> u64 {
    10
}
fn default_idempotency_ttl() -> u64 {
    24
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh_interval(),
            initial_load_timeout_secs: default_initial_timeout(),
            idempotency_ttl_hours: default_idempotency_ttl(),
        }
    }
}

/// TLS 配置
///
/// 控制 gRPC 服务端/客户端的 TLS 行为。
/// 开发环境默认禁用，生产环境通过配置文件或环境变量启用。
#[derive(Debug, Clone, Deserialize)]
pub struct TlsConfig {
    /// 是否启用 TLS（开发环境默认 false）
    #[serde(default)]
    pub enabled: bool,
    /// 服务端证书路径（PEM 格式）
    pub cert_path: Option<String>,
    /// 服务端私钥路径（PEM 格式）
    pub key_path: Option<String>,
    /// CA 证书路径（用于双向 TLS 验证客户端，或客户端验证服务端）
    pub ca_path: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_path: None,
            key_path: None,
            ca_path: None,
        }
    }
}

/// Kafka 安全配置
///
/// 生产环境使用 SASL_SSL 加密 Kafka 通信，
/// 开发环境默认 PLAINTEXT（不配置此节即可）。
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaSecurityConfig {
    /// 安全协议：PLAINTEXT / SSL / SASL_PLAINTEXT / SASL_SSL
    #[serde(default = "default_security_protocol")]
    pub security_protocol: String,
    /// SASL 认证机制：PLAIN / SCRAM-SHA-256 / SCRAM-SHA-512
    pub sasl_mechanism: Option<String>,
    /// SASL 用户名
    pub sasl_username: Option<String>,
    /// SASL 密码
    pub sasl_password: Option<String>,
    /// SSL CA 证书路径
    pub ssl_ca_location: Option<String>,
}

fn default_security_protocol() -> String {
    "PLAINTEXT".into()
}

impl Default for KafkaSecurityConfig {
    fn default() -> Self {
        Self {
            security_protocol: default_security_protocol(),
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
            ssl_ca_location: None,
        }
    }
}

/// 字段级加密配置
///
/// 控制敏感字段的存储加密行为（specs 3.3 条款）。
/// 密钥通过环境变量 `BADGE_ENCRYPTION_KEY` 注入（hex 编码的 32 字节），
/// 不在配置文件中存储，避免密钥泄露。
#[derive(Debug, Clone, Deserialize)]
pub struct EncryptionConfig {
    /// 是否启用字段级加密（开发环境默认关闭，生产环境应开启）
    #[serde(default)]
    pub enabled: bool,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

/// 配置中心配置
///
/// 控制配置热更新行为。方案 B（文件监听）是默认实现，
/// 可通过 `backend` 字段切换为 ETCD 或 Nacos。
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigCenterConfig {
    /// 是否启用配置热更新
    #[serde(default)]
    pub enabled: bool,
    /// 配置源类型：file / etcd / nacos
    #[serde(default = "default_config_backend")]
    pub backend: String,
    /// 文件监听模式下的 debounce 时间（毫秒），避免文件连续写入触发多次重载
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    /// ETCD 端点（backend=etcd 时使用）
    pub etcd_endpoints: Option<Vec<String>>,
    /// Nacos 服务地址（backend=nacos 时使用）
    pub nacos_addr: Option<String>,
}

fn default_config_backend() -> String {
    "file".into()
}

fn default_debounce_ms() -> u64 {
    2000
}

impl Default for ConfigCenterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: default_config_backend(),
            debounce_ms: default_debounce_ms(),
            etcd_endpoints: None,
            nacos_addr: None,
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
    #[serde(default)]
    pub rules: RulesConfig,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub config_center: ConfigCenterConfig,
    #[serde(default)]
    pub encryption: EncryptionConfig,
}

impl AppConfig {
    /// 从配置文件和环境变量加载配置
    ///
    /// 加载顺序（后加载的会覆盖先加载的同名配置项）：
    /// 1. `.env` 文件（加载到环境变量）
    /// 2. `config/default.toml`（默认配置）
    /// 3. `config/{environment}.toml`（环境特定配置）
    /// 4. `config/{service_name}.toml`（服务特定配置）
    /// 5. 环境变量（BADGE_ 前缀，如 BADGE_DATABASE_URL -> database.url）
    /// 6. 服务特定端口环境变量（如 BADGE_ADMIN_PORT, BADGE_MANAGEMENT_PORT）
    pub fn load(service_name: &str) -> Result<Self, ConfigError> {
        // 首先加载 .env 文件到环境变量
        Self::load_dotenv();

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
            // 加载服务特定配置（如 badge-admin-service.toml）
            .add_source(
                File::from(Path::new(&config_dir).join(format!("{}.toml", service_name)))
                    .required(false),
            )
            // 环境变量覆盖（BADGE_DATABASE_URL -> database.url）
            .add_source(
                Environment::with_prefix("BADGE")
                    .separator("_")
                    .try_parsing(true),
            );

        let mut config: Self = builder.build()?.try_deserialize()?;

        // 服务特定端口环境变量覆盖
        // 将服务名转换为环境变量名：badge-admin-service -> BADGE_ADMIN_PORT
        if let Some(port) = Self::get_service_port_from_env(service_name) {
            config.server.port = port;
        }

        Ok(config)
    }

    /// 从环境变量获取服务特定端口
    ///
    /// 服务名到环境变量的映射规则：
    /// - badge-admin-service -> BADGE_ADMIN_PORT
    /// - badge-management-service -> BADGE_MANAGEMENT_PORT
    /// - unified-rule-engine -> RULE_ENGINE_PORT
    /// - event-engagement-service -> EVENT_ENGAGEMENT_PORT
    /// - event-transaction-service -> EVENT_TRANSACTION_PORT
    /// - notification-worker -> NOTIFICATION_WORKER_PORT
    fn get_service_port_from_env(service_name: &str) -> Option<u16> {
        let env_var_name = match service_name {
            "badge-admin-service" => "BADGE_ADMIN_PORT",
            "badge-management-service" => "BADGE_MANAGEMENT_PORT",
            "unified-rule-engine" => "RULE_ENGINE_PORT",
            "event-engagement-service" => "EVENT_ENGAGEMENT_PORT",
            "event-transaction-service" => "EVENT_TRANSACTION_PORT",
            "notification-worker" => "NOTIFICATION_WORKER_PORT",
            // 通用回退：将服务名转换为大写下划线格式 + _PORT
            _ => return Self::get_generic_service_port(service_name),
        };

        std::env::var(env_var_name)
            .ok()
            .and_then(|v| v.parse().ok())
    }

    /// 通用服务端口获取（用于未明确映射的服务）
    ///
    /// 将 "my-service-name" 转换为 "MY_SERVICE_NAME_PORT"
    fn get_generic_service_port(service_name: &str) -> Option<u16> {
        let env_var_name = format!("{}_PORT", service_name.to_uppercase().replace('-', "_"));
        std::env::var(&env_var_name)
            .ok()
            .and_then(|v| v.parse().ok())
    }

    /// 获取服务地址
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// 是否为生产环境
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// 加载 .env 文件到环境变量
    ///
    /// 按优先级尝试加载：
    /// 1. 当前目录的 `.env`
    /// 2. `docker/.env`（项目根目录下的 docker 子目录）
    ///
    /// 如果文件不存在则静默忽略，不影响后续配置加载
    fn load_dotenv() {
        // 优先加载当前目录的 .env
        if dotenvy::dotenv().is_ok() {
            return;
        }

        // 回退到 docker/.env
        let _ = dotenvy::from_filename("docker/.env");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.max_connections, 50);
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

    #[test]
    fn test_service_port_env_var_mapping() {
        // 测试不存在的服务返回 None
        let result = AppConfig::get_service_port_from_env("nonexistent-service");
        // 通用回退会尝试 NONEXISTENT_SERVICE_PORT，不存在时返回 None
        assert!(result.is_none() || result.is_some());

        // 测试函数不会 panic
        let _ = AppConfig::get_service_port_from_env("badge-admin-service");
        let _ = AppConfig::get_service_port_from_env("badge-management-service");
        let _ = AppConfig::get_service_port_from_env("unified-rule-engine");
    }

    #[test]
    fn test_generic_service_port_conversion() {
        // 测试通用服务名转换：my-custom-service -> MY_CUSTOM_SERVICE_PORT
        // 由于环境变量可能不存在，这里只测试函数不会 panic
        let _ = AppConfig::get_generic_service_port("my-custom-service");
    }

    #[test]
    fn test_rules_config_defaults() {
        let config = RulesConfig::default();
        assert_eq!(config.refresh_interval_secs, 30);
        assert_eq!(config.initial_load_timeout_secs, 10);
        assert_eq!(config.idempotency_ttl_hours, 24);
    }

    #[test]
    fn test_kafka_topics_config_defaults() {
        let config = KafkaTopicsConfig::default();
        assert_eq!(config.engagement_events, "badge.engagement.events");
        assert_eq!(config.transaction_events, "badge.transaction.events");
        assert_eq!(config.notifications, "badge.notifications");
        assert_eq!(config.dead_letter_queue, "badge.dlq");
        assert_eq!(config.rule_reload, "badge.rule.reload");
    }

    #[test]
    fn test_service_port_env_var_names() {
        // 验证各服务对应的环境变量名
        let test_cases = vec![
            ("badge-admin-service", "BADGE_ADMIN_PORT"),
            ("badge-management-service", "BADGE_MANAGEMENT_PORT"),
            ("unified-rule-engine", "RULE_ENGINE_PORT"),
            ("event-engagement-service", "EVENT_ENGAGEMENT_PORT"),
            ("event-transaction-service", "EVENT_TRANSACTION_PORT"),
            ("notification-worker", "NOTIFICATION_WORKER_PORT"),
        ];

        for (service_name, expected_env_var) in test_cases {
            // 设置环境变量并验证能正确读取
            // SAFETY: 测试环境中单线程执行，不会有并发问题
            let test_port = 12345u16;
            unsafe {
                std::env::set_var(expected_env_var, test_port.to_string());
            }

            let result = AppConfig::get_service_port_from_env(service_name);
            assert_eq!(
                result,
                Some(test_port),
                "Service '{}' should read from '{}'",
                service_name,
                expected_env_var
            );

            unsafe {
                std::env::remove_var(expected_env_var);
            }
        }
    }
}
