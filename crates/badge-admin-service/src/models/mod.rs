//! B端服务模型模块
//!
//! 包含 B 端特有的实体模型

pub mod operation_log;

// 重新导出常用类型
pub use operation_log::{
    actions, modules, BatchTask, BatchTaskStatus, BatchTaskType, OperationLog,
};
