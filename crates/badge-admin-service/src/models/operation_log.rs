//! 操作日志和批量任务模型
//!
//! B端特有的审计日志和批量处理任务实体

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 操作日志实体
///
/// 记录 B 端所有运营操作，用于审计追溯
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OperationLog {
    pub id: i64,
    /// 操作人 ID
    pub operator_id: String,
    /// 操作人名称（冗余存储，便于查询展示）
    pub operator_name: Option<String>,
    /// 操作模块（badge、rule、grant 等）
    pub module: String,
    /// 操作动作（create、update、delete 等）
    pub action: String,
    /// 操作目标类型
    pub target_type: Option<String>,
    /// 操作目标 ID
    pub target_id: Option<String>,
    /// 变更前数据快照
    pub before_data: Option<serde_json::Value>,
    /// 变更后数据快照
    pub after_data: Option<serde_json::Value>,
    /// 操作者 IP 地址
    pub ip_address: Option<String>,
    /// 客户端 User-Agent
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl OperationLog {
    /// 构建新的操作日志
    pub fn new(
        operator_id: impl Into<String>,
        module: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            id: 0,
            operator_id: operator_id.into(),
            operator_name: None,
            module: module.into(),
            action: action.into(),
            target_type: None,
            target_id: None,
            before_data: None,
            after_data: None,
            ip_address: None,
            user_agent: None,
            created_at: Utc::now(),
        }
    }

    /// 设置操作人名称
    pub fn with_operator_name(mut self, name: impl Into<String>) -> Self {
        self.operator_name = Some(name.into());
        self
    }

    /// 设置操作目标
    pub fn with_target(mut self, target_type: impl Into<String>, target_id: impl Into<String>) -> Self {
        self.target_type = Some(target_type.into());
        self.target_id = Some(target_id.into());
        self
    }

    /// 设置变更数据
    pub fn with_data(mut self, before: Option<serde_json::Value>, after: Option<serde_json::Value>) -> Self {
        self.before_data = before;
        self.after_data = after;
        self
    }

    /// 设置客户端信息
    pub fn with_client_info(mut self, ip_address: Option<String>, user_agent: Option<String>) -> Self {
        self.ip_address = ip_address;
        self.user_agent = user_agent;
        self
    }
}

/// 批量任务实体
///
/// 记录批量发放/取消等异步任务的执行状态
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BatchTask {
    pub id: i64,
    /// 任务类型
    pub task_type: String,
    /// 输入文件地址
    pub file_url: Option<String>,
    /// 总处理条数
    pub total_count: i32,
    /// 成功条数
    pub success_count: i32,
    /// 失败条数
    pub failure_count: i32,
    /// 任务状态
    pub status: String,
    /// 处理进度（0-100）
    pub progress: i32,
    /// 结果文件地址（包含成功/失败详情）
    pub result_file_url: Option<String>,
    /// 错误消息（任务整体失败时）
    pub error_message: Option<String>,
    /// 创建人 ID
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BatchTask {
    /// 创建新的批量任务
    pub fn new(task_type: BatchTaskType, created_by: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            task_type: task_type.as_str().to_string(),
            file_url: None,
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            status: BatchTaskStatus::Pending.as_str().to_string(),
            progress: 0,
            result_file_url: None,
            error_message: None,
            created_by: created_by.into(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 设置输入文件
    pub fn with_file_url(mut self, url: impl Into<String>) -> Self {
        self.file_url = Some(url.into());
        self
    }

    /// 更新任务进度
    pub fn update_progress(&mut self, success: i32, failure: i32, total: i32) {
        self.success_count = success;
        self.failure_count = failure;
        self.total_count = total;
        self.progress = if total > 0 {
            ((success + failure) * 100 / total).min(100)
        } else {
            0
        };
        self.updated_at = Utc::now();
    }

    /// 标记任务为处理中
    pub fn mark_processing(&mut self) {
        self.status = BatchTaskStatus::Processing.as_str().to_string();
        self.updated_at = Utc::now();
    }

    /// 标记任务完成
    pub fn mark_completed(&mut self, result_file_url: Option<String>) {
        self.status = BatchTaskStatus::Completed.as_str().to_string();
        self.progress = 100;
        self.result_file_url = result_file_url;
        self.updated_at = Utc::now();
    }

    /// 标记任务失败
    pub fn mark_failed(&mut self, error_message: impl Into<String>) {
        self.status = BatchTaskStatus::Failed.as_str().to_string();
        self.error_message = Some(error_message.into());
        self.updated_at = Utc::now();
    }

    /// 检查任务是否已完成（成功或失败）
    pub fn is_finished(&self) -> bool {
        self.status == BatchTaskStatus::Completed.as_str()
            || self.status == BatchTaskStatus::Failed.as_str()
    }
}

/// 批量任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchTaskType {
    /// 批量发放
    BatchGrant,
    /// 批量取消
    BatchRevoke,
    /// 数据导出
    DataExport,
}

impl BatchTaskType {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BatchGrant => "batch_grant",
            Self::BatchRevoke => "batch_revoke",
            Self::DataExport => "data_export",
        }
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "batch_grant" => Some(Self::BatchGrant),
            "batch_revoke" => Some(Self::BatchRevoke),
            "data_export" => Some(Self::DataExport),
            _ => None,
        }
    }
}

/// 批量任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchTaskStatus {
    /// 待处理
    Pending,
    /// 处理中
    Processing,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
}

impl BatchTaskStatus {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "processing" => Some(Self::Processing),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// 操作模块常量
pub mod modules {
    pub const CATEGORY: &str = "category";
    pub const SERIES: &str = "series";
    pub const BADGE: &str = "badge";
    pub const RULE: &str = "rule";
    pub const GRANT: &str = "grant";
    pub const REVOKE: &str = "revoke";
}

/// 操作动作常量
pub mod actions {
    pub const CREATE: &str = "create";
    pub const UPDATE: &str = "update";
    pub const DELETE: &str = "delete";
    pub const ENABLE: &str = "enable";
    pub const DISABLE: &str = "disable";
    pub const PUBLISH: &str = "publish";
    pub const ARCHIVE: &str = "archive";
    pub const MANUAL_GRANT: &str = "manual_grant";
    pub const BATCH_GRANT: &str = "batch_grant";
    pub const MANUAL_REVOKE: &str = "manual_revoke";
    pub const BATCH_REVOKE: &str = "batch_revoke";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_log_builder() {
        let log = OperationLog::new("admin001", "badge", "create")
            .with_operator_name("管理员")
            .with_target("badge", "123")
            .with_client_info(Some("192.168.1.1".to_string()), None);

        assert_eq!(log.operator_id, "admin001");
        assert_eq!(log.operator_name, Some("管理员".to_string()));
        assert_eq!(log.module, "badge");
        assert_eq!(log.action, "create");
        assert_eq!(log.target_type, Some("badge".to_string()));
        assert_eq!(log.target_id, Some("123".to_string()));
        assert_eq!(log.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_batch_task_lifecycle() {
        let mut task = BatchTask::new(BatchTaskType::BatchGrant, "admin001")
            .with_file_url("https://oss.example.com/users.csv");

        assert_eq!(task.status, "pending");
        assert_eq!(task.progress, 0);

        task.mark_processing();
        assert_eq!(task.status, "processing");

        task.update_progress(50, 5, 100);
        assert_eq!(task.success_count, 50);
        assert_eq!(task.failure_count, 5);
        assert_eq!(task.progress, 55);

        task.mark_completed(Some("https://oss.example.com/result.csv".to_string()));
        assert_eq!(task.status, "completed");
        assert_eq!(task.progress, 100);
        assert!(task.is_finished());
    }

    #[test]
    fn test_batch_task_failure() {
        let mut task = BatchTask::new(BatchTaskType::DataExport, "admin001");
        task.mark_failed("文件处理失败");

        assert_eq!(task.status, "failed");
        assert_eq!(task.error_message, Some("文件处理失败".to_string()));
        assert!(task.is_finished());
    }

    #[test]
    fn test_batch_task_type_conversion() {
        assert_eq!(BatchTaskType::BatchGrant.as_str(), "batch_grant");
        assert_eq!(
            BatchTaskType::parse("batch_grant"),
            Some(BatchTaskType::BatchGrant)
        );
        assert_eq!(BatchTaskType::parse("unknown"), None);
    }

    #[test]
    fn test_batch_task_status_conversion() {
        assert_eq!(BatchTaskStatus::Processing.as_str(), "processing");
        assert_eq!(
            BatchTaskStatus::parse("completed"),
            Some(BatchTaskStatus::Completed)
        );
    }
}
