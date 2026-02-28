//! REST API 客户端
//!
//! 封装对 badge-admin-service 的 HTTP 调用。

use anyhow::Result;
use reqwest::{Client, Response};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// API 客户端
///
/// 所有 admin API 请求均需要 JWT 认证。
/// 调用 `login()` 获取 token 后，后续请求自动携带 Authorization header。
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    /// JWT token，通过 login() 获取后自动附加到每个请求的 Authorization header
    token: Arc<RwLock<Option<String>>>,
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
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// 登录并缓存 JWT token
    ///
    /// 后续所有 API 请求将自动携带此 token 进行认证。
    /// E2E 测试必须在发起业务请求前调用此方法。
    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        let resp = self
            .client
            .post(self.url("/api/admin/auth/login"))
            .json(&serde_json::json!({
                "username": username,
                "password": password
            }))
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("登录失败 {}: {}", status, error_text));
        }

        let body: serde_json::Value = resp.json().await?;
        let token = body["data"]["token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("登录响应中缺少 token 字段"))?
            .to_string();

        *self.token.write().await = Some(token);
        Ok(())
    }

    // ========== 分类 API ==========

    /// 创建分类
    pub async fn create_category(&self, req: &CreateCategoryRequest) -> Result<CategoryResponse> {
        self.post("/api/admin/categories", req).await
    }

    /// 获取分类列表
    pub async fn list_categories(&self) -> Result<Vec<CategoryResponse>> {
        self.get_paged("/api/admin/categories").await
    }

    /// 获取单个分类
    pub async fn get_category(&self, id: i64) -> Result<CategoryResponse> {
        self.get(&format!("/api/admin/categories/{}", id)).await
    }

    /// 更新分类
    pub async fn update_category(
        &self,
        id: i64,
        req: &UpdateCategoryRequest,
    ) -> Result<CategoryResponse> {
        self.put(&format!("/api/admin/categories/{}", id), req).await
    }

    /// 删除分类
    pub async fn delete_category(&self, id: i64) -> Result<()> {
        self.delete(&format!("/api/admin/categories/{}", id)).await
    }

    // ========== 系列 API ==========

    /// 创建系列
    pub async fn create_series(&self, req: &CreateSeriesRequest) -> Result<SeriesResponse> {
        self.post("/api/admin/series", req).await
    }

    /// 获取系列列表
    pub async fn list_series(&self) -> Result<Vec<SeriesResponse>> {
        self.get_paged("/api/admin/series").await
    }

    // ========== 徽章 API ==========

    /// 创建徽章
    pub async fn create_badge(&self, req: &CreateBadgeRequest) -> Result<BadgeResponse> {
        self.post("/api/admin/badges", req).await
    }

    /// 获取徽章列表
    pub async fn list_badges(&self) -> Result<Vec<BadgeResponse>> {
        self.get_paged("/api/admin/badges").await
    }

    /// 获取单个徽章
    pub async fn get_badge(&self, id: i64) -> Result<BadgeResponse> {
        self.get(&format!("/api/admin/badges/{}", id)).await
    }

    /// 更新徽章
    pub async fn update_badge(&self, id: i64, req: &UpdateBadgeRequest) -> Result<BadgeResponse> {
        self.put(&format!("/api/admin/badges/{}", id), req).await
    }

    /// 发布徽章（草稿 -> 上线）
    pub async fn publish_badge(&self, id: i64) -> Result<BadgeResponse> {
        self.post_empty(&format!("/api/admin/badges/{}/publish", id))
            .await
    }

    /// 下线徽章（上线 -> 下线）
    pub async fn offline_badge(&self, id: i64) -> Result<BadgeResponse> {
        self.post_empty(&format!("/api/admin/badges/{}/offline", id))
            .await
    }

    /// 更新徽章状态（兼容旧测试）
    pub async fn update_badge_status(&self, id: i64, status: &str) -> Result<BadgeResponse> {
        match status.to_lowercase().as_str() {
            "active" => self.publish_badge(id).await,
            "inactive" => self.offline_badge(id).await,
            _ => Err(anyhow::anyhow!("不支持的状态转换: {}", status)),
        }
    }

    // ========== 规则 API ==========

    /// 创建规则
    pub async fn create_rule(&self, req: &CreateRuleRequest) -> Result<RuleResponse> {
        self.post("/api/admin/rules", req).await
    }

    /// 获取规则列表
    pub async fn list_rules(&self) -> Result<Vec<RuleResponse>> {
        self.get_paged("/api/admin/rules").await
    }

    /// 获取单个规则
    pub async fn get_rule(&self, id: i64) -> Result<RuleResponse> {
        self.get(&format!("/api/admin/rules/{}", id)).await
    }

    /// 更新规则
    pub async fn update_rule(&self, id: i64, req: &UpdateRuleRequest) -> Result<RuleResponse> {
        self.put(&format!("/api/admin/rules/{}", id), req).await
    }

    /// 发布（启用）规则
    pub async fn publish_rule(&self, id: i64) -> Result<RuleResponse> {
        self.post_empty(&format!("/api/admin/rules/{}/publish", id)).await
    }

    // ========== 权益 API ==========

    /// 创建权益
    pub async fn create_benefit(&self, req: &CreateBenefitRequest) -> Result<BenefitResponse> {
        self.post("/api/admin/benefits", req).await
    }

    /// 获取权益列表
    pub async fn list_benefits(&self) -> Result<Vec<BenefitResponse>> {
        self.get_paged("/api/admin/benefits").await
    }

    // ========== 用户徽章 API ==========

    /// 获取用户徽章列表
    pub async fn get_user_badges(&self, user_id: &str) -> Result<Vec<UserBadgeResponse>> {
        self.get_paged(&format!("/api/admin/users/{}/badges", user_id)).await
    }

    /// 获取用户权益列表
    pub async fn get_user_benefits(&self, user_id: &str) -> Result<Vec<UserBenefitResponse>> {
        self.get_paged(&format!("/api/admin/users/{}/benefits", user_id)).await
    }

    // ========== 兑换规则 API ==========

    /// 创建兑换规则
    pub async fn create_redemption_rule(
        &self,
        req: &CreateRedemptionRuleRequest,
    ) -> Result<RedemptionRuleResponse> {
        self.post("/api/admin/redemption/rules", req).await
    }

    /// 获取兑换规则列表
    pub async fn list_redemption_rules(&self) -> Result<Vec<RedemptionRuleResponse>> {
        self.get_paged("/api/admin/redemption/rules").await
    }

    /// 按徽章查询兑换规则
    pub async fn list_redemption_rules_by_badge(
        &self,
        badge_id: i64,
    ) -> Result<Vec<RedemptionRuleResponse>> {
        self.get_paged(&format!("/api/admin/redemption/rules?badge_id={}", badge_id))
            .await
    }

    /// 获取单个兑换规则
    pub async fn get_redemption_rule(&self, id: i64) -> Result<RedemptionRuleResponse> {
        self.get(&format!("/api/admin/redemption/rules/{}", id)).await
    }

    // ========== 发放 API ==========

    /// 手动发放徽章
    pub async fn grant_badge(&self, req: &ManualGrantRequest) -> Result<GrantResponse> {
        self.post("/api/admin/grants/manual", req).await
    }

    // ========== 兑换 API ==========

    /// 执行徽章兑换
    pub async fn redeem_badge(&self, req: &RedeemRequest) -> Result<RedeemResponse> {
        self.post("/api/admin/redemption/redeem", req).await
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
        let builder = self
            .client
            .post(self.url("/api/admin/cache/dependencies/refresh"));
        let resp = self.with_auth(builder).await.send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("刷新依赖缓存失败: {}", resp.status()))
        }
    }

    /// 刷新自动权益规则缓存
    ///
    /// 在创建 auto_redeem=true 的兑换规则后调用此方法，
    /// 使规则立即生效，无需等待缓存过期
    pub async fn refresh_auto_benefit_cache(&self) -> Result<()> {
        let builder = self
            .client
            .post(self.url("/api/admin/cache/auto-benefit/refresh"));
        let resp = self.with_auth(builder).await.send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("刷新自动权益缓存失败: {}", resp.status()))
        }
    }

    // ========== 内部方法 ==========

    /// 为请求构建器附加 Bearer 认证头（如果已登录）
    async fn with_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let token_guard = self.token.read().await;
        if let Some(ref token) = *token_guard {
            builder.bearer_auth(token)
        } else {
            builder
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let builder = self.client.get(self.url(path));
        let resp = self.with_auth(builder).await.send().await?;
        self.handle_response(resp).await
    }

    /// 获取分页数据，返回列表中的 items
    async fn get_paged<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let builder = self.client.get(self.url(path));
        let resp = self.with_auth(builder).await.send().await?;
        self.handle_paged_response(resp).await
    }

    async fn post<T: DeserializeOwned, R: Serialize>(&self, path: &str, body: &R) -> Result<T> {
        let builder = self.client.post(self.url(path)).json(body);
        let resp = self.with_auth(builder).await.send().await?;
        self.handle_response(resp).await
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let builder = self.client.post(self.url(path));
        let resp = self.with_auth(builder).await.send().await?;
        self.handle_response(resp).await
    }

    async fn put<T: DeserializeOwned, R: Serialize>(&self, path: &str, body: &R) -> Result<T> {
        let builder = self.client.put(self.url(path)).json(body);
        let resp = self.with_auth(builder).await.send().await?;
        self.handle_response(resp).await
    }

    #[allow(dead_code)] // API 完整性预留，PATCH 方法可能在将来的端点中使用
    async fn patch<T: DeserializeOwned, R: Serialize>(&self, path: &str, body: &R) -> Result<T> {
        let builder = self.client.patch(self.url(path)).json(body);
        let resp = self.with_auth(builder).await.send().await?;
        self.handle_response(resp).await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let builder = self.client.delete(self.url(path));
        let resp = self.with_auth(builder).await.send().await?;
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
            let api_response: ApiResponse<T> = resp.json().await?;
            if api_response.success {
                api_response.data.ok_or_else(|| anyhow::anyhow!("API 返回成功但数据为空"))
            } else {
                Err(anyhow::anyhow!("API 业务错误: {}", api_response.message))
            }
        } else {
            let error_text = resp.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("API 错误 {}: {}", status, error_text))
        }
    }

    /// 处理分页响应，提取 items 列表
    async fn handle_paged_response<T: DeserializeOwned>(&self, resp: Response) -> Result<Vec<T>> {
        let status = resp.status();
        if status.is_success() {
            let api_response: ApiResponse<PageResponse<T>> = resp.json().await?;
            if api_response.success {
                let page_data = api_response.data.ok_or_else(|| anyhow::anyhow!("API 返回成功但数据为空"))?;
                Ok(page_data.items)
            } else {
                Err(anyhow::anyhow!("API 业务错误: {}", api_response.message))
            }
        } else {
            let error_text = resp.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("API 错误 {}: {}", status, error_text))
        }
    }
}

// ========== API 响应包装类型 ==========

/// API 响应包装器，用于解析 badge-admin-service 的标准响应格式
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub code: String,
    pub message: String,
    pub data: Option<T>,
}

/// 分页响应结构，用于解析分页列表接口的数据
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

// ========== 请求/响应类型 ==========

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCategoryRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    // 以下字段用于测试兼容性，实际API不使用
    #[serde(skip)]
    pub description: Option<String>,
    #[serde(skip)]
    pub parent_id: Option<i64>,
}

impl CreateCategoryRequest {
    /// 创建新的分类请求
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            icon_url: None,
            sort_order: None,
            description: None,
            parent_id: None,
        }
    }

    /// 设置描述（测试兼容性）
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// 设置图标 URL
    pub fn with_icon_url(mut self, url: impl Into<String>) -> Self {
        self.icon_url = Some(url.into());
        self
    }

    /// 设置父分类 ID（测试兼容性）
    pub fn with_parent_id(mut self, parent_id: i64) -> Self {
        self.parent_id = Some(parent_id);
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub icon_url: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryResponse {
    pub id: i64,
    pub name: String,
    pub icon_url: Option<String>,
    pub sort_order: i32,
    pub status: String,
    pub badge_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSeriesRequest {
    pub category_id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    // 测试兼容性字段 - 如果使用 theme 会被忽略
    #[serde(skip)]
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesResponse {
    pub id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub sort_order: i32,
    pub status: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub badge_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 徽章资源配置（请求用）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBadgeAssets {
    pub icon_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_icon_url: Option<String>,
}

impl CreateBadgeAssets {
    pub fn new(icon_url: &str) -> Self {
        Self {
            icon_url: icon_url.to_string(),
            image_url: None,
            animation_url: None,
            disabled_icon_url: None,
        }
    }
}

/// 有效期配置（请求用）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateValidityConfig {
    pub validity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relative_days: Option<i32>,
}

impl Default for CreateValidityConfig {
    fn default() -> Self {
        Self {
            validity_type: "PERMANENT".to_string(),
            fixed_date: None,
            relative_days: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBadgeRequest {
    pub series_id: i64,
    pub badge_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obtain_description: Option<String>,
    pub assets: CreateBadgeAssets,
    pub validity_config: CreateValidityConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_supply: Option<i32>,
}

impl CreateBadgeRequest {
    /// 创建徽章请求的快捷方法
    pub fn new(series_id: i64, name: &str, badge_type: &str) -> Self {
        Self {
            series_id,
            badge_type: badge_type.to_string(),
            name: name.to_string(),
            description: None,
            obtain_description: None,
            assets: CreateBadgeAssets::new("https://example.com/default_badge.png"),
            validity_config: CreateValidityConfig::default(),
            max_supply: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn with_icon_url(mut self, url: &str) -> Self {
        self.assets.icon_url = url.to_string();
        self
    }

    pub fn with_max_supply(mut self, supply: i32) -> Self {
        self.max_supply = Some(supply);
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBadgeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_supply: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeResponse {
    pub id: i64,
    pub series_id: i64,
    pub series_name: String,
    pub category_id: i64,
    pub category_name: String,
    pub badge_type: String,
    pub name: String,
    pub description: Option<String>,
    pub obtain_description: Option<String>,
    pub assets: BadgeAssetsResponse,
    pub validity_config: ValidityConfigResponse,
    pub max_supply: Option<i32>,
    pub issued_count: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeAssetsResponse {
    pub icon_url: String,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub animation_url: Option<String>,
    #[serde(default)]
    pub disabled_icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidityConfigResponse {
    pub validity_type: String,
    #[serde(default)]
    pub fixed_date: Option<String>,
    #[serde(default)]
    pub relative_days: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuleRequest {
    pub badge_id: i64,
    pub rule_code: String,
    pub name: String,
    pub event_type: String,
    pub rule_json: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_count_per_user: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_quota: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRuleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_json: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleResponse {
    pub id: i64,
    pub badge_id: i64,
    pub badge_name: String,
    pub rule_json: serde_json::Value,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub max_count_per_user: Option<i32>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBenefitRequest {
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub benefit_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_stock: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub benefit_type: String,
    pub external_id: Option<String>,
    pub external_system: Option<String>,
    pub total_stock: Option<i64>,
    pub remaining_stock: Option<i64>,
    pub status: String,
    pub config: Option<serde_json::Value>,
    pub icon_url: Option<String>,
    pub redeemed_count: i64,
    pub created_at: String,
    pub updated_at: String,
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

/// 所需徽章输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredBadgeInput {
    pub badge_id: i64,
    pub quantity: i32,
}

/// 频率配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequencyConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_user: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_day: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateRedemptionRuleRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub benefit_id: i64,
    pub required_badges: Vec<RequiredBadgeInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_config: Option<FrequencyConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// 是否自动兑换：满足徽章条件时自动发放权益
    #[serde(default)]
    pub auto_redeem: bool,
}

/// 所需徽章响应 DTO
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredBadgeDto {
    pub badge_id: i64,
    pub badge_name: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionRuleResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub benefit_id: i64,
    pub benefit_name: String,
    pub required_badges: Vec<RequiredBadgeDto>,
    pub frequency_config: FrequencyConfig,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
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

#[allow(dead_code)] // 由 serde(default) 使用
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
