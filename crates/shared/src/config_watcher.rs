//! 配置热更新模块
//!
//! 提供 `ConfigWatcher` trait 和多种实现，支持运行时动态更新配置。
//!
//! ## 架构设计
//!
//! ```text
//! ConfigWatcher trait
//!  ├── FileConfigWatcher   — 基于文件系统事件，适合 K8s ConfigMap 挂载（默认）
//!  ├── EtcdConfigWatcher   — 预留，基于 ETCD watch（TODO）
//!  └── NacosConfigWatcher  — 预留，基于 Nacos SDK（TODO）
//! ```
//!
//! 各服务通过 `DynamicConfig` 持有 `Arc<ArcSwap<AppConfig>>`，
//! 读取几乎无开销（一次原子 load），写入通过 watcher 回调自动触发。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::config::AppConfig;

// ============================================================================
// ConfigWatcher trait
// ============================================================================

/// 配置监听器抽象
///
/// 不同配置源（文件、ETCD、Nacos）实现此 trait，
/// 当配置变更时通过 `watch::Sender` 推送新配置。
#[async_trait]
pub trait ConfigWatcher: Send + Sync {
    /// 启动监听，配置变更时通过 sender 推送
    async fn start(&self) -> Result<()>;

    /// 停止监听并释放资源
    async fn stop(&self) -> Result<()>;
}

// ============================================================================
// DynamicConfig — 线程安全的动态配置容器
// ============================================================================

/// 动态配置容器
///
/// 使用 `ArcSwap` 实现近零开销的读取（仅一次原子 load），
/// 配合 `watch` channel 让消费方异步等待配置变更通知。
#[derive(Clone)]
pub struct DynamicConfig {
    /// 当前配置快照，读取端通过 ArcSwap::load 获取（无锁）
    current: Arc<ArcSwap<AppConfig>>,
    /// 配置变更通知 channel 的发送端，仅 watcher 内部使用
    tx: watch::Sender<Arc<AppConfig>>,
    /// 配置变更通知 channel 的接收端，消费方通过 clone 订阅
    rx: watch::Receiver<Arc<AppConfig>>,
}

impl DynamicConfig {
    /// 用初始配置创建 DynamicConfig
    pub fn new(config: AppConfig) -> Self {
        let config = Arc::new(config);
        let (tx, rx) = watch::channel(config.clone());
        Self {
            current: Arc::new(ArcSwap::from(config)),
            tx,
            rx,
        }
    }

    /// 获取当前配置快照（近零开销的原子 load）
    pub fn load(&self) -> Arc<AppConfig> {
        self.current.load_full()
    }

    /// 获取 watch receiver，用于异步等待配置变更
    pub fn subscribe(&self) -> watch::Receiver<Arc<AppConfig>> {
        self.rx.clone()
    }

    /// 更新配置（由 watcher 回调调用）
    ///
    /// 同时更新 ArcSwap 快照和 watch channel，
    /// 保证 load() 读取和 subscribe() 通知的一致性。
    pub fn update(&self, new_config: AppConfig) {
        let new_config = Arc::new(new_config);
        self.current.store(new_config.clone());
        // send 失败说明没有 receiver，属于正常情况（服务关闭阶段）
        let _ = self.tx.send(new_config);
    }

    /// 获取内部 ArcSwap 引用（供需要直接访问的场景）
    pub fn inner(&self) -> &Arc<ArcSwap<AppConfig>> {
        &self.current
    }
}

// ============================================================================
// FileConfigWatcher — 基于文件系统事件的配置热更新
// ============================================================================

/// 基于文件系统事件的配置监听器
///
/// 使用 `notify` crate 监听配置文件变化，
/// 文件写入后经 debounce 窗口去抖再重新加载并推送。
///
/// 适用场景：
/// - K8s ConfigMap 挂载到容器文件系统
/// - 本地开发时手动编辑配置文件
pub struct FileConfigWatcher {
    /// 服务名，用于调用 AppConfig::load 重载配置
    service_name: String,
    /// 监听的配置文件路径
    watch_path: PathBuf,
    /// debounce 窗口，避免文件连续写入触发多次重载
    debounce: Duration,
    /// 动态配置容器，变更时自动推送
    dynamic_config: DynamicConfig,
    /// 用于通知 watcher 循环退出
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

impl FileConfigWatcher {
    pub fn new(
        service_name: &str,
        watch_path: impl AsRef<Path>,
        debounce: Duration,
        dynamic_config: DynamicConfig,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            service_name: service_name.to_string(),
            watch_path: watch_path.as_ref().to_path_buf(),
            debounce,
            dynamic_config,
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// 从 AppConfig 的 config_center 配置节自动构建
    pub fn from_config(
        service_name: &str,
        config: &AppConfig,
        dynamic_config: DynamicConfig,
    ) -> Self {
        let config_dir =
            std::env::var("CONFIG_DIR").unwrap_or_else(|_| "config".to_string());
        let debounce = Duration::from_millis(config.config_center.debounce_ms);
        Self::new(service_name, &config_dir, debounce, dynamic_config)
    }

}

#[async_trait]
impl ConfigWatcher for FileConfigWatcher {
    async fn start(&self) -> Result<()> {
        use notify::{RecursiveMode, Watcher};

        let watch_path = self.watch_path.clone();
        let debounce = self.debounce;
        let service_name = self.service_name.clone();
        let dynamic_config = self.dynamic_config.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();

        // notify 事件通过 channel 转发到 tokio 异步任务
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<()>(16);

        // 在阻塞线程中创建 watcher，因为 notify 使用系统原生 API
        let _watcher_handle = tokio::task::spawn_blocking({
            let watch_path = watch_path.clone();
            let event_tx = event_tx.clone();
            move || -> Result<notify::RecommendedWatcher> {
                let mut watcher = notify::recommended_watcher(
                    move |res: Result<notify::Event, notify::Error>| {
                        match res {
                            Ok(event) => {
                                // 只关心写入/创建/删除事件
                                use notify::EventKind;
                                match event.kind {
                                    EventKind::Modify(_)
                                    | EventKind::Create(_)
                                    | EventKind::Remove(_) => {
                                        let _ = event_tx.try_send(());
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "文件监听器事件错误");
                            }
                        }
                    },
                )
                .context("创建文件监听器失败")?;

                watcher
                    .watch(&watch_path, RecursiveMode::NonRecursive)
                    .context("启动文件监听失败")?;

                info!(path = %watch_path.display(), "配置文件监听已启动");
                Ok(watcher)
            }
        });

        // 异步 debounce 循环：收到文件事件后等待 debounce 窗口再重载
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // 收到文件变更事件
                    Some(()) = event_rx.recv() => {
                        // Debounce：等待窗口期，丢弃窗口内的后续事件
                        tokio::time::sleep(debounce).await;
                        // 清空积压的重复事件
                        while event_rx.try_recv().is_ok() {}

                        match AppConfig::load(&service_name) {
                            Ok(new_config) => {
                                info!(
                                    service = %service_name,
                                    "配置文件变更，已重新加载"
                                );
                                dynamic_config.update(new_config);
                            }
                            Err(e) => {
                                error!(
                                    service = %service_name,
                                    error = %e,
                                    "配置文件重新加载失败，保留当前配置"
                                );
                            }
                        }
                    }
                    // 收到关闭信号
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("配置文件监听已停止");
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let _ = self.shutdown_tx.send(true);
        Ok(())
    }
}

// ============================================================================
// EtcdConfigWatcher — 预留接口
// ============================================================================

/// 基于 ETCD watch 的配置监听器（预留）
///
/// TODO: 引入 etcd-client crate，监听 /badge/config/{service_name} key 的变化
pub struct EtcdConfigWatcher {
    #[allow(dead_code)]
    endpoints: Vec<String>,
    #[allow(dead_code)]
    dynamic_config: DynamicConfig,
}

impl EtcdConfigWatcher {
    pub fn new(endpoints: Vec<String>, dynamic_config: DynamicConfig) -> Self {
        Self {
            endpoints,
            dynamic_config,
        }
    }
}

#[async_trait]
impl ConfigWatcher for EtcdConfigWatcher {
    async fn start(&self) -> Result<()> {
        // TODO: 使用 etcd-client crate 实现
        // 1. 连接 ETCD 集群
        // 2. 监听配置 key 变化
        // 3. 变更时解析并推送到 DynamicConfig
        warn!("EtcdConfigWatcher 尚未实现，请使用 FileConfigWatcher");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// NacosConfigWatcher — 预留接口
// ============================================================================

/// 基于 Nacos SDK 的配置监听器（预留）
///
/// TODO: 引入 nacos-sdk crate，监听 Nacos 配置中心的变更事件
pub struct NacosConfigWatcher {
    #[allow(dead_code)]
    addr: String,
    #[allow(dead_code)]
    dynamic_config: DynamicConfig,
}

impl NacosConfigWatcher {
    pub fn new(addr: String, dynamic_config: DynamicConfig) -> Self {
        Self {
            addr,
            dynamic_config,
        }
    }
}

#[async_trait]
impl ConfigWatcher for NacosConfigWatcher {
    async fn start(&self) -> Result<()> {
        // TODO: 使用 nacos-sdk crate 实现
        // 1. 连接 Nacos 服务
        // 2. 订阅 dataId=badge-{service_name}, group=DEFAULT_GROUP
        // 3. 变更时解析并推送到 DynamicConfig
        warn!("NacosConfigWatcher 尚未实现，请使用 FileConfigWatcher");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// 工厂函数
// ============================================================================

/// 根据配置自动创建对应的 ConfigWatcher
///
/// 读取 `config_center.backend` 决定使用哪种实现：
/// - "file"  → FileConfigWatcher（默认）
/// - "etcd"  → EtcdConfigWatcher（预留）
/// - "nacos" → NacosConfigWatcher（预留）
pub fn create_watcher(
    service_name: &str,
    config: &AppConfig,
    dynamic_config: DynamicConfig,
) -> Box<dyn ConfigWatcher> {
    match config.config_center.backend.as_str() {
        "etcd" => {
            let endpoints = config
                .config_center
                .etcd_endpoints
                .clone()
                .unwrap_or_else(|| vec!["http://localhost:2379".to_string()]);
            Box::new(EtcdConfigWatcher::new(endpoints, dynamic_config))
        }
        "nacos" => {
            let addr = config
                .config_center
                .nacos_addr
                .clone()
                .unwrap_or_else(|| "http://localhost:8848".to_string());
            Box::new(NacosConfigWatcher::new(addr, dynamic_config))
        }
        _ => Box::new(FileConfigWatcher::from_config(
            service_name,
            config,
            dynamic_config,
        )),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_config_load_and_update() {
        let initial = AppConfig::default();
        let dc = DynamicConfig::new(initial);

        // 初始配置的端口应为默认值
        assert_eq!(dc.load().server.port, 8080);

        // 更新配置后应能读到新值
        let mut updated = AppConfig::default();
        updated.server.port = 9090;
        dc.update(updated);
        assert_eq!(dc.load().server.port, 9090);
    }

    #[test]
    fn test_dynamic_config_subscribe() {
        let dc = DynamicConfig::new(AppConfig::default());
        let mut rx = dc.subscribe();

        // subscriber 应能获取初始值
        assert_eq!(rx.borrow().server.port, 8080);

        // 更新后 subscriber 应能看到新值
        let mut updated = AppConfig::default();
        updated.server.port = 3000;
        dc.update(updated);
        assert_eq!(rx.borrow_and_update().server.port, 3000);
    }

    #[test]
    fn test_config_center_config_defaults() {
        let cc = super::super::config::ConfigCenterConfig::default();
        assert!(!cc.enabled);
        assert_eq!(cc.backend, "file");
        assert_eq!(cc.debounce_ms, 2000);
        assert!(cc.etcd_endpoints.is_none());
        assert!(cc.nacos_addr.is_none());
    }

    #[test]
    fn test_create_watcher_file() {
        let config = AppConfig::default();
        let dc = DynamicConfig::new(config.clone());
        let _watcher = create_watcher("test-service", &config, dc);
        // 默认 backend=file，不应 panic
    }

    #[test]
    fn test_create_watcher_etcd() {
        let mut config = AppConfig::default();
        config.config_center.backend = "etcd".to_string();
        let dc = DynamicConfig::new(config.clone());
        let _watcher = create_watcher("test-service", &config, dc);
    }

    #[test]
    fn test_create_watcher_nacos() {
        let mut config = AppConfig::default();
        config.config_center.backend = "nacos".to_string();
        let dc = DynamicConfig::new(config.clone());
        let _watcher = create_watcher("test-service", &config, dc);
    }
}
