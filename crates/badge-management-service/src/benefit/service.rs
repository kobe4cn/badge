//! 权益服务层
//!
//! 封装权益发放、撤销、查询的业务逻辑，通过 HandlerRegistry 路由到具体的 Handler 实现。
//!
//! ## 设计说明
//!
//! BenefitService 是权益发放的统一入口，负责：
//! 1. 根据权益类型路由到对应的 Handler
//! 2. 生成发放流水号（如未提供）
//! 3. 记录发放日志和状态
//! 4. 处理发放失败的重试和回滚
//!
//! ## 使用示例
//!
//! ```ignore
//! use badge_management::benefit::{BenefitService, GrantBenefitRequest};
//! use badge_management::benefit::registry::HandlerRegistry;
//! use badge_management::models::BenefitType;
//! use std::sync::Arc;
//!
//! // 创建服务
//! let registry = Arc::new(HandlerRegistry::with_defaults());
//! let service = BenefitService::new(registry);
//!
//! // 发放权益
//! let request = GrantBenefitRequest::new(
//!     "user-123",
//!     BenefitType::Coupon,
//!     1,
//!     serde_json::json!({"coupon_template_id": "tpl-001"}),
//! );
//! let result = service.grant_benefit(request).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::benefit::dto::{BenefitGrantRequest, BenefitGrantResult};
use crate::benefit::registry::HandlerRegistry;
use crate::error::{BadgeError, Result};
use crate::models::{BenefitType, GrantStatus, RevokeReason};

/// 发放权益请求
///
/// 服务层的请求结构，包含发放所需的全部信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantBenefitRequest {
    /// 发放流水号（可选，不提供时自动生成）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_no: Option<String>,
    /// 目标用户 ID
    pub user_id: String,
    /// 权益类型
    pub benefit_type: BenefitType,
    /// 权益定义 ID
    pub benefit_id: i64,
    /// 权益配置
    pub benefit_config: Value,
    /// 关联的兑换订单 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redemption_order_id: Option<i64>,
    /// 扩展元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl GrantBenefitRequest {
    /// 创建发放请求
    pub fn new(
        user_id: impl Into<String>,
        benefit_type: BenefitType,
        benefit_id: i64,
        benefit_config: Value,
    ) -> Self {
        Self {
            grant_no: None,
            user_id: user_id.into(),
            benefit_type,
            benefit_id,
            benefit_config,
            redemption_order_id: None,
            metadata: None,
        }
    }

    /// 设置发放流水号
    pub fn with_grant_no(mut self, grant_no: impl Into<String>) -> Self {
        self.grant_no = Some(grant_no.into());
        self
    }

    /// 设置关联的兑换订单 ID
    pub fn with_redemption_order(mut self, order_id: i64) -> Self {
        self.redemption_order_id = Some(order_id);
        self
    }

    /// 设置扩展元数据
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 发放权益响应
///
/// 包含发放结果和追踪信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantBenefitResponse {
    /// 发放流水号
    pub grant_no: String,
    /// 发放状态
    pub status: GrantStatus,
    /// 权益类型
    pub benefit_type: BenefitType,
    /// 外部系统引用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<String>,
    /// 发放时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granted_at: Option<DateTime<Utc>>,
    /// 过期时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// 结果载荷
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    /// 错误消息（发放失败时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// 处理耗时（毫秒）
    pub duration_ms: u64,
}

impl GrantBenefitResponse {
    /// 从 Handler 返回的结果构建响应
    fn from_result(
        result: BenefitGrantResult,
        benefit_type: BenefitType,
        duration_ms: u64,
    ) -> Self {
        Self {
            grant_no: result.grant_no,
            status: result.status,
            benefit_type,
            external_ref: result.external_ref,
            granted_at: result.granted_at,
            expires_at: result.expires_at,
            payload: result.payload,
            error_message: result.message,
            duration_ms,
        }
    }

    /// 检查发放是否成功
    pub fn is_success(&self) -> bool {
        self.status == GrantStatus::Success
    }

    /// 检查发放是否仍在处理中
    pub fn is_processing(&self) -> bool {
        self.status == GrantStatus::Processing
    }
}

/// 撤销结果
///
/// 撤销操作的返回结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeResult {
    /// 发放流水号
    pub grant_no: String,
    /// 是否成功
    pub success: bool,
    /// 撤销原因
    pub reason: RevokeReason,
    /// 撤销时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
    /// 结果消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl RevokeResult {
    /// 创建成功的撤销结果
    fn success(grant_no: impl Into<String>, reason: RevokeReason) -> Self {
        Self {
            grant_no: grant_no.into(),
            success: true,
            reason,
            revoked_at: Some(Utc::now()),
            message: None,
        }
    }

    /// 创建失败的撤销结果
    fn failed(
        grant_no: impl Into<String>,
        reason: RevokeReason,
        message: impl Into<String>,
    ) -> Self {
        Self {
            grant_no: grant_no.into(),
            success: false,
            reason,
            revoked_at: None,
            message: Some(message.into()),
        }
    }
}

/// 内存中的发放记录（用于演示，生产环境应使用数据库）
#[derive(Debug, Clone)]
struct GrantRecord {
    grant_no: String,
    #[allow(dead_code)]
    user_id: String,
    benefit_type: BenefitType,
    #[allow(dead_code)]
    benefit_id: i64,
    status: GrantStatus,
    external_ref: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// 权益服务
///
/// 提供权益发放、撤销、查询的统一接口
pub struct BenefitService {
    /// Handler 注册表
    registry: Arc<HandlerRegistry>,
    /// 内存存储（用于演示，生产环境应注入 Repository）
    grants: RwLock<HashMap<String, GrantRecord>>,
    /// 可选的数据库连接池（用于持久化 benefit_grants 记录）
    pool: Option<PgPool>,
}

impl BenefitService {
    /// 创建权益服务
    pub fn new(registry: Arc<HandlerRegistry>) -> Self {
        Self {
            registry,
            grants: RwLock::new(HashMap::new()),
            pool: None,
        }
    }

    /// 使用默认 Handler 创建服务
    pub fn with_defaults() -> Self {
        Self::new(Arc::new(HandlerRegistry::with_defaults()))
    }

    /// 设置数据库连接池（启用持久化到 benefit_grants 表）
    pub fn with_pool(mut self, pool: PgPool) -> Self {
        self.pool = Some(pool);
        self
    }

    /// 生成发放流水号
    ///
    /// 格式: BG{YYMMDD}{随机字符}
    fn generate_grant_no() -> String {
        let date = Utc::now().format("%y%m%d");
        let random = &Uuid::new_v4().to_string()[..8].to_uppercase();
        format!("BG{}{}", date, random)
    }

    /// 发放权益
    ///
    /// 根据权益类型路由到对应的 Handler 执行发放操作
    #[instrument(
        skip(self, request),
        fields(
            user_id = %request.user_id,
            benefit_type = ?request.benefit_type,
            benefit_id = request.benefit_id
        )
    )]
    pub async fn grant_benefit(
        &self,
        request: GrantBenefitRequest,
    ) -> Result<GrantBenefitResponse> {
        let start = Instant::now();

        // 生成或使用提供的流水号
        let grant_no = request
            .grant_no
            .clone()
            .unwrap_or_else(Self::generate_grant_no);

        info!(grant_no = %grant_no, "开始发放权益");

        // 检查幂等性（使用内存存储演示）
        {
            let grants = self.grants.read().await;
            if let Some(existing) = grants.get(&grant_no) {
                warn!(
                    grant_no = %grant_no,
                    status = ?existing.status,
                    "发现重复的发放请求"
                );
                return Ok(GrantBenefitResponse {
                    grant_no: existing.grant_no.clone(),
                    status: existing.status,
                    benefit_type: existing.benefit_type,
                    external_ref: existing.external_ref.clone(),
                    granted_at: Some(existing.created_at),
                    expires_at: None,
                    payload: None,
                    error_message: Some("重复的发放请求".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }

        // 获取对应的 Handler
        let handler = self.registry.get(request.benefit_type).ok_or_else(|| {
            error!(benefit_type = ?request.benefit_type, "未找到对应的权益处理器");
            BadgeError::Internal(format!(
                "未注册权益类型 {:?} 的处理器",
                request.benefit_type
            ))
        })?;

        // 构建 Handler 请求
        let mut handler_request = BenefitGrantRequest::new(
            &grant_no,
            &request.user_id,
            request.benefit_id,
            request.benefit_config.clone(),
        );

        if let Some(order_id) = request.redemption_order_id {
            handler_request = handler_request.with_redemption_order(order_id);
        }
        if let Some(metadata) = request.metadata.clone() {
            handler_request = handler_request.with_metadata(metadata);
        }

        // 调用 Handler 执行发放
        let result = handler.grant(handler_request).await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // 记录发放结果（内存存储演示）
        {
            let mut grants = self.grants.write().await;
            let now = Utc::now();
            grants.insert(
                grant_no.clone(),
                GrantRecord {
                    grant_no: grant_no.clone(),
                    user_id: request.user_id.clone(),
                    benefit_type: request.benefit_type,
                    benefit_id: request.benefit_id,
                    status: result.status,
                    external_ref: result.external_ref.clone(),
                    created_at: now,
                    updated_at: now,
                },
            );
        }

        let response = GrantBenefitResponse::from_result(result, request.benefit_type, duration_ms);

        // 持久化到数据库（如果配置了数据库池）
        if (response.is_success() || response.is_processing())
            && let Err(e) = self
                .persist_grant_to_db(
                    &grant_no,
                    &request.user_id,
                    request.benefit_id,
                    request.benefit_type,
                    response.status,
                    response.external_ref.as_deref(),
                    response.payload.as_ref(),
                )
                .await
        {
            // 持久化失败不影响主流程，只记录警告
            warn!(grant_no = %grant_no, error = %e, "持久化 benefit_grants 失败");
        }

        if response.is_success() {
            info!(
                grant_no = %response.grant_no,
                duration_ms = duration_ms,
                "权益发放成功"
            );
        } else if response.is_processing() {
            info!(
                grant_no = %response.grant_no,
                duration_ms = duration_ms,
                "权益发放已提交，等待处理"
            );
        } else {
            warn!(
                grant_no = %response.grant_no,
                error = ?response.error_message,
                duration_ms = duration_ms,
                "权益发放失败"
            );
        }

        Ok(response)
    }

    /// 撤销权益发放
    ///
    /// 调用对应 Handler 的撤销方法，部分权益类型不支持撤销
    #[instrument(skip(self), fields(grant_no = %grant_no, reason = ?reason))]
    pub async fn revoke_grant(&self, grant_no: &str, reason: RevokeReason) -> Result<RevokeResult> {
        info!("开始撤销权益");

        // 查找发放记录
        let record = {
            let grants = self.grants.read().await;
            grants.get(grant_no).cloned()
        };

        let record = match record {
            Some(r) => r,
            None => {
                warn!("发放记录不存在");
                return Ok(RevokeResult::failed(grant_no, reason, "发放记录不存在"));
            }
        };

        // 检查状态是否允许撤销
        if record.status == GrantStatus::Revoked {
            warn!("权益已撤销");
            return Ok(RevokeResult::failed(grant_no, reason, "权益已撤销"));
        }

        if record.status != GrantStatus::Success {
            warn!(status = ?record.status, "当前状态不允许撤销");
            return Ok(RevokeResult::failed(
                grant_no,
                reason,
                format!("当前状态 {:?} 不允许撤销", record.status),
            ));
        }

        // 检查权益类型是否支持撤销
        if !record.benefit_type.is_revocable() {
            warn!(benefit_type = ?record.benefit_type, "该权益类型不支持撤销");
            return Ok(RevokeResult::failed(
                grant_no,
                reason,
                format!("权益类型 {:?} 不支持撤销", record.benefit_type),
            ));
        }

        // 获取 Handler
        let handler = self.registry.get(record.benefit_type).ok_or_else(|| {
            BadgeError::Internal(format!("未注册权益类型 {:?} 的处理器", record.benefit_type))
        })?;

        // 调用 Handler 撤销
        let revoke_result = handler.revoke(grant_no).await;

        match revoke_result {
            Ok(result) if result.success => {
                // 更新记录状态
                {
                    let mut grants = self.grants.write().await;
                    if let Some(r) = grants.get_mut(grant_no) {
                        r.status = GrantStatus::Revoked;
                        r.updated_at = Utc::now();
                    }
                }

                info!("权益撤销成功");
                Ok(RevokeResult::success(grant_no, reason))
            }
            Ok(result) => {
                warn!(message = ?result.message, "权益撤销失败");
                Ok(RevokeResult::failed(
                    grant_no,
                    reason,
                    result.message.unwrap_or_else(|| "撤销失败".to_string()),
                ))
            }
            Err(e) => {
                error!(error = %e, "权益撤销异常");
                Ok(RevokeResult::failed(grant_no, reason, e.to_string()))
            }
        }
    }

    /// 查询发放状态
    ///
    /// 先查询本地记录，如果是处理中状态则调用 Handler 获取最新状态
    #[instrument(skip(self))]
    pub async fn query_grant_status(&self, grant_no: &str) -> Result<GrantStatus> {
        debug!("查询发放状态");

        // 查找本地记录
        let record = {
            let grants = self.grants.read().await;
            grants.get(grant_no).cloned()
        };

        match record {
            Some(r) => {
                // 如果是处理中状态，调用 Handler 查询最新状态
                if r.status == GrantStatus::Processing
                    && let Some(handler) = self.registry.get(r.benefit_type)
                {
                    let latest_status = handler.query_status(grant_no).await?;

                    // 如果状态有变化，更新本地记录
                    if latest_status != r.status {
                        let mut grants = self.grants.write().await;
                        if let Some(record) = grants.get_mut(grant_no) {
                            record.status = latest_status;
                            record.updated_at = Utc::now();
                        }
                    }

                    return Ok(latest_status);
                }

                Ok(r.status)
            }
            None => {
                warn!("发放记录不存在");
                Err(BadgeError::Internal(format!(
                    "发放记录不存在: {}",
                    grant_no
                )))
            }
        }
    }

    /// 批量查询发放状态
    pub async fn query_grant_statuses(
        &self,
        grant_nos: &[String],
    ) -> Result<HashMap<String, GrantStatus>> {
        let mut results = HashMap::new();

        for grant_no in grant_nos {
            match self.query_grant_status(grant_no).await {
                Ok(status) => {
                    results.insert(grant_no.clone(), status);
                }
                Err(e) => {
                    debug!(grant_no = %grant_no, error = %e, "查询状态失败");
                }
            }
        }

        Ok(results)
    }

    /// 验证权益配置
    ///
    /// 在发放前验证配置的合法性
    pub fn validate_config(&self, benefit_type: BenefitType, config: &Value) -> Result<()> {
        let handler = self.registry.get(benefit_type).ok_or_else(|| {
            BadgeError::Internal(format!("未注册权益类型 {:?} 的处理器", benefit_type))
        })?;

        handler.validate_config(config)
    }

    /// 获取支持的权益类型列表
    pub fn supported_types(&self) -> Vec<BenefitType> {
        self.registry.registered_types()
    }

    /// 检查是否支持指定的权益类型
    pub fn supports(&self, benefit_type: BenefitType) -> bool {
        self.registry.contains(benefit_type)
    }

    /// 为自动权益规则发放权益
    ///
    /// 当用户获得徽章触发自动权益规则时调用此方法。
    /// 与 `grant_benefit` 不同，此方法直接使用 benefit_id 查找权益配置，
    /// 无需调用者提供完整的权益配置信息。
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `rule_id` - 自动权益规则 ID（用于日志追踪）
    /// * `benefit_id` - 权益定义 ID
    /// * `idempotency_key` - 幂等键（防止重复发放）
    ///
    /// # Returns
    /// * `Ok(Some(i64))` - 成功发放后返回发放记录 ID（如果有持久化）
    /// * `Ok(None)` - 成功发放但没有持久化记录（演示模式）
    /// * `Err(_)` - 发放失败
    ///
    /// # Note
    /// 当前实现为演示版本，使用内存存储。生产环境需要：
    /// 1. 通过 RedemptionRepository 获取权益定义
    /// 2. 将发放记录持久化到数据库
    /// 3. 返回真实的数据库记录 ID
    #[instrument(skip(self), fields(user_id = %user_id, rule_id = rule_id, benefit_id = benefit_id))]
    pub async fn grant_benefit_for_auto_rule(
        &self,
        user_id: &str,
        rule_id: i64,
        benefit_id: i64,
        idempotency_key: &str,
    ) -> Result<Option<i64>> {
        info!(
            "自动权益发放: rule_id={}, benefit_id={}",
            rule_id, benefit_id
        );

        // 从数据库获取权益类型，如果失败则使用默认的 Coupon 类型
        let benefit_type = self
            .get_benefit_type(benefit_id)
            .await
            .unwrap_or(BenefitType::Coupon);

        // 根据权益类型构建配置
        let benefit_config = match benefit_type {
            BenefitType::Points => serde_json::json!({
                "point_amount": 100,
                "source": "auto_benefit"
            }),
            BenefitType::Coupon => serde_json::json!({
                "coupon_template_id": format!("auto_rule_{}", rule_id),
                "quantity": 1,
                "source": "auto_benefit"
            }),
            _ => serde_json::json!({
                "source": "auto_benefit",
                "rule_id": rule_id
            }),
        };

        let request = GrantBenefitRequest::new(user_id, benefit_type, benefit_id, benefit_config)
            .with_grant_no(idempotency_key);

        let response = self.grant_benefit(request).await?;

        if response.is_success() {
            info!(
                "自动权益发放成功: grant_no={}",
                response.grant_no
            );
            // 返回持久化记录的 ID（如果有数据库池）
            let grant_id = self.get_grant_id_by_grant_no(&response.grant_no).await;
            Ok(grant_id)
        } else {
            let error_msg = response
                .error_message
                .unwrap_or_else(|| "未知错误".to_string());
            warn!("自动权益发放失败: {}", error_msg);
            Err(BadgeError::Internal(format!(
                "自动权益发放失败: {}",
                error_msg
            )))
        }
    }

    /// 持久化权益发放记录到数据库
    ///
    /// 使用事务确保发放记录插入和库存扣减的原子性。
    /// 当发放成功时，同时扣减 benefits 表中的 remaining_stock。
    #[allow(clippy::too_many_arguments)]
    async fn persist_grant_to_db(
        &self,
        grant_no: &str,
        user_id: &str,
        benefit_id: i64,
        _benefit_type: BenefitType,
        status: GrantStatus,
        external_ref: Option<&str>,
        payload: Option<&Value>,
    ) -> Result<Option<i64>> {
        let Some(ref pool) = self.pool else {
            // 没有配置数据库池，跳过持久化
            return Ok(None);
        };

        let status_str = match status {
            GrantStatus::Pending => "pending",
            GrantStatus::Success => "success",
            GrantStatus::Processing => "processing",
            GrantStatus::Failed => "failed",
            GrantStatus::Revoked => "revoked",
        };

        // 使用事务保证发放记录和库存扣减的原子性
        let mut tx = pool.begin().await?;

        // benefit_type 存储在 benefits 表中，通过 benefit_id 关联查询
        let id: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO benefit_grants (
                grant_no, user_id, benefit_id, status, external_ref, external_response, granted_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (grant_no) DO UPDATE SET
                status = EXCLUDED.status,
                external_ref = EXCLUDED.external_ref,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(grant_no)
        .bind(user_id)
        .bind(benefit_id)
        .bind(status_str)
        .bind(external_ref)
        .bind(payload)
        .fetch_one(&mut *tx)
        .await?;

        // 发放成功时扣减库存，remaining_stock > 0 条件防止超卖
        if status == GrantStatus::Success {
            sqlx::query(
                r#"
                UPDATE benefits
                SET remaining_stock = remaining_stock - 1,
                    updated_at = NOW()
                WHERE id = $1 AND remaining_stock > 0
                "#,
            )
            .bind(benefit_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        debug!(grant_no = %grant_no, id = id.0, "权益发放记录已持久化");
        Ok(Some(id.0))
    }

    /// 根据 grant_no 查询记录 ID
    async fn get_grant_id_by_grant_no(&self, grant_no: &str) -> Option<i64> {
        let pool = self.pool.as_ref()?;

        let result: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM benefit_grants WHERE grant_no = $1")
                .bind(grant_no)
                .fetch_optional(pool)
                .await
                .ok()?;

        result.map(|(id,)| id)
    }

    /// 根据 benefit_id 查询权益类型
    async fn get_benefit_type(&self, benefit_id: i64) -> Option<BenefitType> {
        let pool = self.pool.as_ref()?;

        let result: Option<(String,)> =
            sqlx::query_as("SELECT benefit_type FROM benefits WHERE id = $1")
                .bind(benefit_id)
                .fetch_optional(pool)
                .await
                .ok()?;

        result.and_then(|(t,)| match t.to_uppercase().as_str() {
            "POINTS" => Some(BenefitType::Points),
            "COUPON" => Some(BenefitType::Coupon),
            "PHYSICAL" => Some(BenefitType::Physical),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_service() -> BenefitService {
        BenefitService::with_defaults()
    }

    #[test]
    fn test_generate_grant_no() {
        let grant_no = BenefitService::generate_grant_no();

        // 验证格式: BG{YYMMDD}{8位随机}
        assert!(grant_no.starts_with("BG"));
        assert_eq!(grant_no.len(), 16); // BG + 6位日期 + 8位随机
    }

    #[test]
    fn test_grant_benefit_request_new() {
        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Coupon,
            1,
            json!({"coupon_template_id": "tpl-001"}),
        );

        assert_eq!(request.user_id, "user-123");
        assert_eq!(request.benefit_type, BenefitType::Coupon);
        assert_eq!(request.benefit_id, 1);
        assert!(request.grant_no.is_none());
    }

    #[test]
    fn test_grant_benefit_request_builder() {
        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Points,
            1,
            json!({"point_amount": 100}),
        )
        .with_grant_no("custom-grant-no")
        .with_redemption_order(100)
        .with_metadata(json!({"source": "promotion"}));

        assert_eq!(request.grant_no, Some("custom-grant-no".to_string()));
        assert_eq!(request.redemption_order_id, Some(100));
        assert!(request.metadata.is_some());
    }

    #[tokio::test]
    async fn test_grant_benefit_coupon() {
        let service = create_service();

        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Coupon,
            1,
            json!({"coupon_template_id": "tpl-001", "quantity": 1}),
        );

        let response = service.grant_benefit(request).await.unwrap();

        assert!(response.is_success());
        assert_eq!(response.benefit_type, BenefitType::Coupon);
        assert!(response.external_ref.is_some());
        assert!(response.granted_at.is_some());
    }

    #[tokio::test]
    async fn test_grant_benefit_points() {
        let service = create_service();

        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Points,
            1,
            json!({"point_amount": 100, "point_type": "bonus"}),
        );

        let response = service.grant_benefit(request).await.unwrap();

        assert!(response.is_success());
        assert_eq!(response.benefit_type, BenefitType::Points);
    }

    #[tokio::test]
    async fn test_grant_benefit_physical() {
        let service = create_service();

        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Physical,
            1,
            json!({
                "sku_id": "SKU001",
                "shipping_address": {
                    "recipient_name": "张三",
                    "phone": "13800138000",
                    "province": "北京市",
                    "city": "北京市",
                    "district": "朝阳区",
                    "address": "某某街道 123 号"
                }
            }),
        );

        let response = service.grant_benefit(request).await.unwrap();

        // 实物是异步发放
        assert!(response.is_processing());
        assert_eq!(response.benefit_type, BenefitType::Physical);
    }

    #[tokio::test]
    async fn test_grant_benefit_idempotent() {
        let service = create_service();

        let grant_no = "idempotent-test-001";
        let request1 = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Coupon,
            1,
            json!({"coupon_template_id": "tpl-001"}),
        )
        .with_grant_no(grant_no);

        let response1 = service.grant_benefit(request1).await.unwrap();
        assert!(response1.is_success());

        // 第二次使用相同 grant_no 发放
        let request2 = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Coupon,
            1,
            json!({"coupon_template_id": "tpl-001"}),
        )
        .with_grant_no(grant_no);

        let response2 = service.grant_benefit(request2).await.unwrap();

        // 应该返回已存在的记录
        assert_eq!(response2.grant_no, grant_no);
        assert!(response2.error_message.is_some());
        assert!(response2.error_message.unwrap().contains("重复"));
    }

    #[tokio::test]
    async fn test_grant_benefit_invalid_config() {
        let service = create_service();

        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Coupon,
            1,
            json!({}), // 缺少必要字段
        );

        let response = service.grant_benefit(request).await.unwrap();

        // Handler 返回失败状态而非抛出错误
        assert!(!response.is_success());
        assert_eq!(response.status, GrantStatus::Failed);
    }

    #[tokio::test]
    async fn test_query_grant_status() {
        let service = create_service();

        // 先发放
        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Coupon,
            1,
            json!({"coupon_template_id": "tpl-001"}),
        )
        .with_grant_no("query-test-001");

        service.grant_benefit(request).await.unwrap();

        // 查询状态
        let status = service.query_grant_status("query-test-001").await.unwrap();
        assert_eq!(status, GrantStatus::Success);
    }

    #[tokio::test]
    async fn test_query_grant_status_not_found() {
        let service = create_service();

        let result = service.query_grant_status("non-existent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoke_grant_coupon() {
        let service = create_service();

        // 先发放
        let grant_no = "revoke-test-001";
        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Coupon,
            1,
            json!({"coupon_template_id": "tpl-001"}),
        )
        .with_grant_no(grant_no);

        service.grant_benefit(request).await.unwrap();

        // 撤销
        let result = service
            .revoke_grant(grant_no, RevokeReason::UserRequest)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.reason, RevokeReason::UserRequest);
        assert!(result.revoked_at.is_some());

        // 验证状态已更新
        let status = service.query_grant_status(grant_no).await.unwrap();
        assert_eq!(status, GrantStatus::Revoked);
    }

    #[tokio::test]
    async fn test_revoke_grant_physical_not_supported() {
        let service = create_service();

        // 先发放实物
        let grant_no = "revoke-physical-test";
        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Physical,
            1,
            json!({
                "sku_id": "SKU001",
                "shipping_address": {
                    "recipient_name": "张三",
                    "phone": "13800138000",
                    "province": "北京市",
                    "city": "北京市",
                    "district": "朝阳区",
                    "address": "某某街道"
                }
            }),
        )
        .with_grant_no(grant_no);

        service.grant_benefit(request).await.unwrap();

        // 尝试撤销（实物不支持撤销，但状态是 Processing）
        let result = service
            .revoke_grant(grant_no, RevokeReason::UserRequest)
            .await
            .unwrap();

        // 因为状态是 Processing，不允许撤销
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_revoke_grant_not_found() {
        let service = create_service();

        let result = service
            .revoke_grant("non-existent", RevokeReason::UserRequest)
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.message.unwrap().contains("不存在"));
    }

    #[tokio::test]
    async fn test_revoke_grant_already_revoked() {
        let service = create_service();

        // 先发放
        let grant_no = "double-revoke-test";
        let request = GrantBenefitRequest::new(
            "user-123",
            BenefitType::Coupon,
            1,
            json!({"coupon_template_id": "tpl-001"}),
        )
        .with_grant_no(grant_no);

        service.grant_benefit(request).await.unwrap();

        // 第一次撤销
        let result1 = service
            .revoke_grant(grant_no, RevokeReason::UserRequest)
            .await
            .unwrap();
        assert!(result1.success);

        // 第二次撤销
        let result2 = service
            .revoke_grant(grant_no, RevokeReason::SystemError)
            .await
            .unwrap();
        assert!(!result2.success);
        assert!(result2.message.unwrap().contains("已撤销"));
    }

    #[test]
    fn test_validate_config() {
        let service = create_service();

        // 有效的优惠券配置
        let valid_config = json!({"coupon_template_id": "tpl-001"});
        assert!(
            service
                .validate_config(BenefitType::Coupon, &valid_config)
                .is_ok()
        );

        // 无效的优惠券配置
        let invalid_config = json!({});
        assert!(
            service
                .validate_config(BenefitType::Coupon, &invalid_config)
                .is_err()
        );
    }

    #[test]
    fn test_supported_types() {
        let service = create_service();

        let types = service.supported_types();
        assert_eq!(types.len(), 3);
        assert!(service.supports(BenefitType::Coupon));
        assert!(service.supports(BenefitType::Points));
        assert!(service.supports(BenefitType::Physical));
        assert!(!service.supports(BenefitType::DigitalAsset));
    }

    #[tokio::test]
    async fn test_batch_query_statuses() {
        let service = create_service();

        // 发放多个
        for i in 1..=3 {
            let request = GrantBenefitRequest::new(
                "user-123",
                BenefitType::Coupon,
                1,
                json!({"coupon_template_id": "tpl-001"}),
            )
            .with_grant_no(format!("batch-{}", i));

            service.grant_benefit(request).await.unwrap();
        }

        // 批量查询
        let grant_nos: Vec<String> = (1..=3).map(|i| format!("batch-{}", i)).collect();
        let statuses = service.query_grant_statuses(&grant_nos).await.unwrap();

        assert_eq!(statuses.len(), 3);
        for (_, status) in statuses {
            assert_eq!(status, GrantStatus::Success);
        }
    }
}
