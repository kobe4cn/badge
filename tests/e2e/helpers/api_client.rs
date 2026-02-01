//! REST API 客户端
//!
//! 封装对 badge-admin-service 的 HTTP 调用。

use anyhow::Result;
use reqwest::{Client, Response};
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;

/// API 客户端
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("创建 HTTP 客户端失败");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    // ========== 分类 API ==========

    /// 创建分类
    pub async fn create_category(&self, req: &CreateCategoryRequest) -> Result<CategoryResponse> {
        self.post("/api/categories", req).await
    }

    /// 获取分类列表
    pub async fn list_categories(&self) -> Result<Vec<CategoryResponse>> {
        self.get("/api/categories").await
    }

    /// 获取单个分类
    pub async fn get_category(&self, id: i64) -> Result<CategoryResponse> {
        self.get(&format!("/api/categories/{}", id)).await
    }

    /// 更新分类
    pub async fn update_category(
        &self,
        id: i64,
        req: &UpdateCategoryRequest,
    ) -> Result<CategoryResponse> {
        self.put(&format!("/api/categories/{}", id), req).await
    }

    /// 删除分类
    pub async fn delete_category(&self, id: i64) -> Result<()> {
        self.delete(&format!("/api/categories/{}", id)).await
    }

    // ========== 系列 API ==========

    /// 创建系列
    pub async fn create_series(&self, req: &CreateSeriesRequest) -> Result<SeriesResponse> {
        self.post("/api/series", req).await
    }

    /// 获取系列列表
    pub async fn list_series(&self) -> Result<Vec<SeriesResponse>> {
        self.get("/api/series").await
    }

    // ========== 徽章 API ==========

    /// 创建徽章
    pub async fn create_badge(&self, req: &CreateBadgeRequest) -> Result<BadgeResponse> {
        self.post("/api/badges", req).await
    }

    /// 获取徽章列表
    pub async fn list_badges(&self) -> Result<Vec<BadgeResponse>> {
        self.get("/api/badges").await
    }

    /// 获取单个徽章
    pub async fn get_badge(&self, id: i64) -> Result<BadgeResponse> {
        self.get(&format!("/api/badges/{}", id)).await
    }

    /// 更新徽章
    pub async fn update_badge(&self, id: i64, req: &UpdateBadgeRequest) -> Result<BadgeResponse> {
        self.put(&format!("/api/badges/{}", id), req).await
    }

    /// 更新徽章状态
    pub async fn update_badge_status(&self, id: i64, status: &str) -> Result<BadgeResponse> {
        self.patch(
            &format!("/api/badges/{}/status", id),
            &serde_json::json!({ "status": status }),
        )
        .await
    }

    // ========== 规则 API ==========

    /// 创建规则
    pub async fn create_rule(&self, req: &CreateRuleRequest) -> Result<RuleResponse> {
        self.post("/api/rules", req).await
    }

    /// 获取规则列表
    pub async fn list_rules(&self) -> Result<Vec<RuleResponse>> {
        self.get("/api/rules").await
    }

    /// 获取单个规则
    pub async fn get_rule(&self, id: i64) -> Result<RuleResponse> {
        self.get(&format!("/api/rules/{}", id)).await
    }

    /// 更新规则
    pub async fn update_rule(&self, id: i64, req: &UpdateRuleRequest) -> Result<RuleResponse> {
        self.put(&format!("/api/rules/{}", id), req).await
    }

    // ========== 权益 API ==========

    /// 创建权益
    pub async fn create_benefit(&self, req: &CreateBenefitRequest) -> Result<BenefitResponse> {
        self.post("/api/benefits", req).await
    }

    /// 获取权益列表
    pub async fn list_benefits(&self) -> Result<Vec<BenefitResponse>> {
        self.get("/api/benefits").await
    }

    // ========== 用户徽章 API ==========

    /// 获取用户徽章列表
    pub async fn get_user_badges(&self, user_id: &str) -> Result<Vec<UserBadgeResponse>> {
        self.get(&format!("/api/users/{}/badges", user_id)).await
    }

    /// 获取用户权益列表
    pub async fn get_user_benefits(&self, user_id: &str) -> Result<Vec<UserBenefitResponse>> {
        self.get(&format!("/api/users/{}/benefits", user_id)).await
    }

    // ========== 兑换规则 API ==========

    /// 创建兑换规则
    pub async fn create_redemption_rule(
        &self,
        req: &CreateRedemptionRuleRequest,
    ) -> Result<RedemptionRuleResponse> {
        self.post("/api/redemption-rules", req).await
    }

    /// 获取兑换规则列表
    pub async fn list_redemption_rules(&self) -> Result<Vec<RedemptionRuleResponse>> {
        self.get("/api/redemption-rules").await
    }

    /// 按徽章查询兑换规则
    pub async fn list_redemption_rules_by_badge(
        &self,
        badge_id: i64,
    ) -> Result<Vec<RedemptionRuleResponse>> {
        self.get(&format!("/api/redemption-rules?badge_id={}", badge_id))
            .await
    }

    /// 获取单个兑换规则
    pub async fn get_redemption_rule(&self, id: i64) -> Result<RedemptionRuleResponse> {
        self.get(&format!("/api/redemption-rules/{}", id)).await
    }

    // ========== 发放 API ==========

    /// 手动发放徽章
    pub async fn grant_badge(&self, req: &ManualGrantRequest) -> Result<GrantResponse> {
        self.post("/api/admin/grants/manual", req).await
    }

    // ========== 兑换 API ==========

    /// 执行徽章兑换
    pub async fn redeem_badge(&self, req: &RedeemRequest) -> Result<RedeemResponse> {
        self.post("/api/redemptions", req).await
    }

    // ========== 依赖关系 API ==========

    /// 创建徽章依赖关系
    pub async fn create_dependency(
        &self,
        badge_id: i64,
        req: &CreateDependencyRequest,
    ) -> Result<DependencyResponse> {
        self.post(&format!("/api/admin/badges/{}/dependencies", badge_id), req)
            .await
    }

    /// 获取徽章的依赖关系列表
    pub async fn list_dependencies(&self, badge_id: i64) -> Result<Vec<DependencyResponse>> {
        self.get(&format!("/api/admin/badges/{}/dependencies", badge_id))
            .await
    }

    /// 删除依赖关系
    pub async fn delete_dependency(&self, id: i64) -> Result<()> {
        self.delete(&format!("/api/admin/dependencies/{}", id))
            .await
    }

    /// 刷新依赖缓存
    pub async fn refresh_dependency_cache(&self) -> Result<()> {
        let resp = self
            .client
            .post(self.url("/api/admin/cache/dependencies/refresh"))
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("刷新依赖缓存失败: {}", resp.status()))
        }
    }

    // ========== 内部方法 ==========

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.client.get(self.url(path)).send().await?;
        self.handle_response(resp).await
    }

    async fn post<T: DeserializeOwned, R: Serialize>(&self, path: &str, body: &R) -> Result<T> {
        let resp = self.client.post(self.url(path)).json(body).send().await?;
        self.handle_response(resp).await
    }

    async fn put<T: DeserializeOwned, R: Serialize>(&self, path: &str, body: &R) -> Result<T> {
        let resp = self.client.put(self.url(path)).json(body).send().await?;
        self.handle_response(resp).await
    }

    async fn patch<T: DeserializeOwned, R: Serialize>(&self, path: &str, body: &R) -> Result<T> {
        let resp = self.client.patch(self.url(path)).json(body).send().await?;
        self.handle_response(resp).await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let resp = self.client.delete(self.url(path)).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("删除失败: {}", resp.status()))
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn handle_response<T: DeserializeOwned>(&self, resp: Response) -> Result<T> {
        let status = resp.status();
        if status.is_success() {
            Ok(resp.json().await?)
        } else {
            let error_text = resp.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("API 错误 {}: {}", status, error_text))
        }
    }
}

// ========== 请求/响应类型 ==========

#[derive(Debug, Clone, Serialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub parent_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateSeriesRequest {
    pub category_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeriesResponse {
    pub id: i64,
    pub category_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateBadgeRequest {
    pub series_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub badge_type: String,
    pub icon_url: Option<String>,
    pub max_supply: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateBadgeRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub max_supply: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BadgeResponse {
    pub id: i64,
    pub series_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub badge_type: String,
    pub icon_url: Option<String>,
    pub max_supply: Option<i64>,
    pub issued_count: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateRuleRequest {
    pub badge_id: i64,
    pub rule_code: String,
    pub name: String,
    pub event_type: String,
    pub rule_json: serde_json::Value,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub max_count_per_user: Option<i32>,
    pub global_quota: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateRuleRequest {
    pub name: Option<String>,
    pub rule_json: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleResponse {
    pub id: i64,
    pub badge_id: i64,
    pub rule_code: String,
    pub name: String,
    pub event_type: String,
    pub rule_json: serde_json::Value,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateBenefitRequest {
    pub name: String,
    pub benefit_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BenefitResponse {
    pub id: i64,
    pub name: String,
    pub benefit_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserBadgeResponse {
    pub id: i64,
    pub user_id: String,
    pub badge_id: i64,
    pub badge_name: String,
    pub status: String,
    pub quantity: i32,
    pub acquired_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserBenefitResponse {
    pub grant_no: String,
    pub benefit_type: String,
    pub status: String,
    pub granted_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateRedemptionRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub benefit_id: i64,
    pub required_badges: serde_json::Value,
    pub auto_redeem: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedemptionRuleResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub benefit_id: i64,
    pub required_badges: serde_json::Value,
    pub auto_redeem: bool,
    pub status: String,
}

// ========== 依赖关系 API ==========

/// 创建徽章依赖关系请求
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDependencyRequest {
    /// 依赖的徽章 ID
    pub depends_on_badge_id: i64,
    /// 依赖类型：prerequisite, consume, exclusive
    pub dependency_type: String,
    /// 需要的数量
    #[serde(default = "default_quantity")]
    pub required_quantity: i32,
    /// 互斥组 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_group_id: Option<String>,
    /// 是否自动触发级联
    #[serde(default)]
    pub auto_trigger: bool,
    /// 优先级
    #[serde(default)]
    pub priority: i32,
    /// 依赖组 ID
    pub dependency_group_id: String,
}

fn default_quantity() -> i32 {
    1
}

/// 依赖关系响应
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyResponse {
    pub id: i64,
    pub badge_id: i64,
    pub depends_on_badge_id: i64,
    pub dependency_type: String,
    pub required_quantity: i32,
    pub exclusive_group_id: Option<String>,
    pub auto_trigger: bool,
    pub priority: i32,
    pub dependency_group_id: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

use serde::Deserialize;

// ========== 发放 API ==========

/// 手动发放徽章请求
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualGrantRequest {
    pub user_id: String,
    pub badge_id: i64,
    pub quantity: i32,
    pub reason: String,
}

/// 发放结果响应
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantResponse {
    pub user_id: String,
    pub badge_id: i64,
    pub badge_name: String,
    pub quantity: i32,
    pub source_ref_id: String,
}

// ========== 兑换 API ==========

/// 兑换请求
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemRequest {
    pub user_id: String,
    pub redemption_rule_id: i64,
}

/// 兑换结果响应
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemResponse {
    pub success: bool,
    pub order_no: Option<String>,
    pub message: Option<String>,
}
