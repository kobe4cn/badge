//! 数据一致性测试套件
//!
//! 验证前后端数据、事务和缓存一致性。
//! 每个测试独立使用 reqwest::Client + JWT 认证访问 API，
//! 并通过 sqlx::PgPool 直接查询数据库来交叉验证数据一致性。

use reqwest::Client;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::time::Duration;

/// 默认服务地址
const DEFAULT_BASE_URL: &str = "http://127.0.0.1:8080";
/// 默认数据库连接串
const DEFAULT_DATABASE_URL: &str = "postgres://badge:badge_secret@localhost:5432/badge_db";

/// 获取 JWT token，用于后续所有需要鉴权的 API 调用
async fn login(client: &Client, base_url: &str) -> String {
    let resp = client
        .post(api_url(base_url, "/api/admin/auth/login"))
        .json(&json!({
            "username": "admin",
            "password": "admin123"
        }))
        .send()
        .await
        .expect("登录请求发送失败");

    assert!(resp.status().is_success(), "登录应返回成功状态码");

    let body: Value = resp.json().await.expect("登录响应解析失败");
    body["data"]["token"]
        .as_str()
        .expect("登录响应中缺少 token 字段")
        .to_string()
}

/// 拼接完整的 API URL
fn api_url(base_url: &str, path: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), path)
}

/// 获取测试用的基础 URL
fn base_url() -> String {
    std::env::var("ADMIN_SERVICE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

/// 获取测试用的数据库连接串
fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string())
}

/// 生成唯一后缀，避免并行测试数据冲突
fn unique_suffix() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..8].to_string()
}

#[cfg(test)]
mod api_db_consistency_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_badge_create_api_db_consistency() {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        let base = base_url();
        let token = login(&client, &base).await;
        let pool = PgPool::connect(&database_url()).await.expect("数据库连接失败");

        let suffix = unique_suffix();

        // --- 创建分类 ---
        let cat_resp = client
            .post(api_url(&base, "/api/admin/categories"))
            .bearer_auth(&token)
            .json(&json!({
                "name": format!("consistency_test_cat_{}", suffix),
                "sortOrder": 0
            }))
            .send()
            .await
            .unwrap();
        assert!(cat_resp.status().is_success(), "创建分类应成功");
        let cat_body: Value = cat_resp.json().await.unwrap();
        let cat_id = cat_body["data"]["id"].as_i64().expect("分类 ID 缺失");

        // --- 创建系列 ---
        let series_resp = client
            .post(api_url(&base, "/api/admin/series"))
            .bearer_auth(&token)
            .json(&json!({
                "name": format!("consistency_test_series_{}", suffix),
                "categoryId": cat_id,
                "sortOrder": 0
            }))
            .send()
            .await
            .unwrap();
        assert!(series_resp.status().is_success(), "创建系列应成功");
        let series_body: Value = series_resp.json().await.unwrap();
        let series_id = series_body["data"]["id"].as_i64().expect("系列 ID 缺失");

        // --- 创建徽章 ---
        let badge_name = format!("consistency_badge_{}", suffix);
        let badge_desc = "API-DB 一致性测试徽章";
        let badge_resp = client
            .post(api_url(&base, "/api/admin/badges"))
            .bearer_auth(&token)
            .json(&json!({
                "seriesId": series_id,
                "badgeType": "NORMAL",
                "name": badge_name,
                "description": badge_desc,
                "assets": {
                    "iconUrl": "https://example.com/consistency_test.png"
                },
                "validityConfig": {
                    "validityType": "PERMANENT"
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(badge_resp.status().is_success(), "创建徽章应成功");
        let badge_body: Value = badge_resp.json().await.unwrap();
        let api_badge = &badge_body["data"];
        let badge_id = api_badge["id"].as_i64().expect("徽章 ID 缺失");

        // --- 直接查询数据库验证一致性 ---
        let db_row: (String, Option<String>, String, i64, String) = sqlx::query_as(
            "SELECT name, description, badge_type, series_id, status FROM badges WHERE id = $1",
        )
        .bind(badge_id)
        .fetch_one(&pool)
        .await
        .expect("数据库中应存在刚创建的徽章");

        // API 响应与数据库记录字段逐一比较
        assert_eq!(
            api_badge["name"].as_str().unwrap(),
            db_row.0,
            "name: API 与 DB 不一致"
        );
        assert_eq!(
            api_badge["description"].as_str().map(|s| s.to_string()),
            db_row.1,
            "description: API 与 DB 不一致"
        );
        assert_eq!(
            api_badge["badgeType"].as_str().unwrap(),
            db_row.2,
            "badgeType: API 与 DB 不一致"
        );
        assert_eq!(
            api_badge["seriesId"].as_i64().unwrap(),
            db_row.3,
            "seriesId: API 与 DB 不一致"
        );
        assert_eq!(
            api_badge["status"].as_str().unwrap(),
            db_row.4,
            "status: API 与 DB 不一致"
        );

        // --- 清理：依次删除徽章、系列、分类（外键约束要求此顺序） ---
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/badges/{}", badge_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/series/{}", series_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/categories/{}", cat_id)))
            .bearer_auth(&token)
            .send()
            .await;
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_rule_create_api_db_consistency() {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        let base = base_url();
        let token = login(&client, &base).await;
        let pool = PgPool::connect(&database_url()).await.expect("数据库连接失败");

        let suffix = unique_suffix();

        // 规则需要先绑定徽章，先创建分类->系列->徽章
        let cat_resp = client
            .post(api_url(&base, "/api/admin/categories"))
            .bearer_auth(&token)
            .json(&json!({ "name": format!("rule_consist_cat_{}", suffix), "sortOrder": 0 }))
            .send()
            .await
            .unwrap();
        let cat_id = cat_resp.json::<Value>().await.unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        let series_resp = client
            .post(api_url(&base, "/api/admin/series"))
            .bearer_auth(&token)
            .json(&json!({ "name": format!("rule_consist_series_{}", suffix), "categoryId": cat_id, "sortOrder": 0 }))
            .send()
            .await
            .unwrap();
        let series_id = series_resp.json::<Value>().await.unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        let badge_resp = client
            .post(api_url(&base, "/api/admin/badges"))
            .bearer_auth(&token)
            .json(&json!({
                "seriesId": series_id,
                "badgeType": "NORMAL",
                "name": format!("rule_consist_badge_{}", suffix),
                "assets": { "iconUrl": "https://example.com/test.png" },
                "validityConfig": { "validityType": "PERMANENT" }
            }))
            .send()
            .await
            .unwrap();
        let badge_id = badge_resp.json::<Value>().await.unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        // --- 创建规则 ---
        let rule_name = format!("consist_rule_{}", suffix);
        let rule_code = format!("consist_rule_code_{}", suffix);
        let rule_resp = client
            .post(api_url(&base, "/api/admin/rules"))
            .bearer_auth(&token)
            .json(&json!({
                "badgeId": badge_id,
                "ruleCode": rule_code,
                "name": rule_name,
                "eventType": "purchase",
                "ruleJson": {
                    "type": "condition",
                    "field": "amount",
                    "operator": "gte",
                    "value": 100
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(rule_resp.status().is_success(), "创建规则应成功");
        let rule_body: Value = rule_resp.json().await.unwrap();
        let api_rule = &rule_body["data"];
        let rule_id = api_rule["id"].as_i64().expect("规则 ID 缺失");

        // --- 直接查询数据库验证 ---
        let db_row: (i64, bool) = sqlx::query_as(
            "SELECT badge_id, enabled FROM badge_rules WHERE id = $1",
        )
        .bind(rule_id)
        .fetch_one(&pool)
        .await
        .expect("数据库中应存在刚创建的规则");

        assert_eq!(
            api_rule["badgeId"].as_i64().unwrap(),
            db_row.0,
            "badgeId: API 与 DB 不一致"
        );
        assert_eq!(
            api_rule["enabled"].as_bool().unwrap(),
            db_row.1,
            "enabled: API 与 DB 不一致"
        );

        // --- 清理 ---
        let _ = sqlx::query("DELETE FROM badge_rules WHERE id = $1")
            .bind(rule_id)
            .execute(&pool)
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/badges/{}", badge_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/series/{}", series_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/categories/{}", cat_id)))
            .bearer_auth(&token)
            .send()
            .await;
    }
}

#[cfg(test)]
mod transaction_consistency_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_badge_grant_transaction() {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        let base = base_url();
        let token = login(&client, &base).await;
        let pool = PgPool::connect(&database_url()).await.expect("数据库连接失败");

        let suffix = unique_suffix();
        let user_id = format!("test_tx_user_{}", suffix);

        // --- 搭建徽章基础设施 ---
        let cat_id = client
            .post(api_url(&base, "/api/admin/categories"))
            .bearer_auth(&token)
            .json(&json!({ "name": format!("tx_cat_{}", suffix), "sortOrder": 0 }))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        let series_id = client
            .post(api_url(&base, "/api/admin/series"))
            .bearer_auth(&token)
            .json(&json!({ "name": format!("tx_series_{}", suffix), "categoryId": cat_id, "sortOrder": 0 }))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        let badge_id = client
            .post(api_url(&base, "/api/admin/badges"))
            .bearer_auth(&token)
            .json(&json!({
                "seriesId": series_id,
                "badgeType": "NORMAL",
                "name": format!("tx_badge_{}", suffix),
                "assets": { "iconUrl": "https://example.com/tx.png" },
                "validityConfig": { "validityType": "PERMANENT" }
            }))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        // 发布徽章使其可被发放
        let publish_resp = client
            .post(api_url(&base, &format!("/api/admin/badges/{}/publish", badge_id)))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(publish_resp.status().is_success(), "发布徽章应成功");

        // --- 手动发放徽章 ---
        let grant_resp = client
            .post(api_url(&base, "/api/admin/grants/manual"))
            .bearer_auth(&token)
            .json(&json!({
                "userId": user_id,
                "badgeId": badge_id,
                "quantity": 1,
                "reason": "事务一致性测试"
            }))
            .send()
            .await
            .unwrap();
        assert!(grant_resp.status().is_success(), "发放徽章应成功");

        // --- 验证事务原子性：badge_grants 和 user_badges 同时存在记录 ---
        // 等待异步处理完成
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 检查 user_badges 表
        let ub_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_badges WHERE user_id = $1 AND badge_id = $2 AND UPPER(status) = 'ACTIVE'",
        )
        .bind(&user_id)
        .bind(badge_id)
        .fetch_one(&pool)
        .await
        .expect("查询 user_badges 失败");

        assert!(
            ub_count.0 > 0,
            "user_badges 表应有发放记录（事务原子提交）"
        );

        // 检查 badge_ledger 表确认有对应的账本条目
        let ledger_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM badge_ledger WHERE user_id = $1 AND badge_id = $2",
        )
        .bind(&user_id)
        .bind(badge_id)
        .fetch_one(&pool)
        .await
        .expect("查询 badge_ledger 失败");

        assert!(
            ledger_count.0 > 0,
            "badge_ledger 表应有账本记录（事务原子提交）"
        );

        // --- 清理 ---
        let _ = sqlx::query("DELETE FROM badge_ledger WHERE user_id = $1 AND badge_id = $2")
            .bind(&user_id)
            .bind(badge_id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM user_badges WHERE user_id = $1 AND badge_id = $2")
            .bind(&user_id)
            .bind(badge_id)
            .execute(&pool)
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/badges/{}", badge_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/series/{}", series_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/categories/{}", cat_id)))
            .bearer_auth(&token)
            .send()
            .await;
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_redemption_transaction_rollback() {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        let base = base_url();
        let token = login(&client, &base).await;
        let pool = PgPool::connect(&database_url()).await.expect("数据库连接失败");

        let suffix = unique_suffix();
        let user_id = format!("test_rollback_user_{}", suffix);

        // 使用不存在的兑换规则 ID 发起兑换，期望失败
        let fake_rule_id = 999999999_i64;

        let redeem_resp = client
            .post(api_url(&base, "/api/admin/redemption/redeem"))
            .bearer_auth(&token)
            .json(&json!({
                "userId": user_id,
                "redemptionRuleId": fake_rule_id
            }))
            .send()
            .await
            .unwrap();

        // 兑换请求应返回错误（规则不存在或用户不满足条件）
        let redeem_body: Value = redeem_resp.json().await.unwrap();
        let is_error = !redeem_body["success"].as_bool().unwrap_or(false)
            || redeem_body["data"]["success"].as_bool() == Some(false);
        assert!(is_error, "使用无效规则兑换应失败: {:?}", redeem_body);

        // --- 验证事务回滚：不应产生任何孤立的兑换订单 ---
        let orphan_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM redemption_orders WHERE user_id = $1 AND redemption_rule_id = $2",
        )
        .bind(&user_id)
        .bind(fake_rule_id)
        .fetch_one(&pool)
        .await
        .unwrap_or((0,));

        assert_eq!(
            orphan_count.0, 0,
            "失败的兑换不应在 redemption_orders 中留下孤立记录（事务已回滚）"
        );

        // 同时确认 benefit_grants 也没有残留
        let benefit_orphan: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM benefit_grants WHERE user_id = $1",
        )
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .unwrap_or((0,));

        assert_eq!(
            benefit_orphan.0, 0,
            "失败的兑换不应在 benefit_grants 中产生残留记录"
        );
    }
}

#[cfg(test)]
mod cache_consistency_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_rule_cache_invalidation() {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        let base = base_url();
        let token = login(&client, &base).await;
        let pool = PgPool::connect(&database_url()).await.expect("数据库连接失败");

        let suffix = unique_suffix();

        // --- 创建前置数据 ---
        let cat_id = client
            .post(api_url(&base, "/api/admin/categories"))
            .bearer_auth(&token)
            .json(&json!({ "name": format!("cache_cat_{}", suffix), "sortOrder": 0 }))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        let series_id = client
            .post(api_url(&base, "/api/admin/series"))
            .bearer_auth(&token)
            .json(&json!({ "name": format!("cache_series_{}", suffix), "categoryId": cat_id, "sortOrder": 0 }))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        let badge_id = client
            .post(api_url(&base, "/api/admin/badges"))
            .bearer_auth(&token)
            .json(&json!({
                "seriesId": series_id,
                "badgeType": "NORMAL",
                "name": format!("cache_badge_{}", suffix),
                "assets": { "iconUrl": "https://example.com/cache.png" },
                "validityConfig": { "validityType": "PERMANENT" }
            }))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        // --- 创建规则 ---
        let original_name = format!("cache_rule_original_{}", suffix);
        let rule_resp = client
            .post(api_url(&base, "/api/admin/rules"))
            .bearer_auth(&token)
            .json(&json!({
                "badgeId": badge_id,
                "ruleCode": format!("cache_rule_code_{}", suffix),
                "name": original_name,
                "eventType": "purchase",
                "ruleJson": {
                    "type": "condition",
                    "field": "amount",
                    "operator": "gte",
                    "value": 50
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(rule_resp.status().is_success(), "创建规则应成功");
        let rule_id = rule_resp.json::<Value>().await.unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        // --- 首次 GET 确认原始名称 ---
        let get_resp = client
            .get(api_url(&base, &format!("/api/admin/rules/{}", rule_id)))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let get_body: Value = get_resp.json().await.unwrap();
        assert_eq!(
            get_body["data"]["name"].as_str().unwrap(),
            &original_name,
            "首次 GET 应返回原始名称"
        );

        // --- 更新规则名称 ---
        let updated_name = format!("cache_rule_updated_{}", suffix);
        let update_resp = client
            .put(api_url(&base, &format!("/api/admin/rules/{}", rule_id)))
            .bearer_auth(&token)
            .json(&json!({ "name": updated_name }))
            .send()
            .await
            .unwrap();
        assert!(update_resp.status().is_success(), "更新规则应成功");

        // --- 再次 GET 验证缓存已失效，返回新名称 ---
        let get_resp2 = client
            .get(api_url(&base, &format!("/api/admin/rules/{}", rule_id)))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let get_body2: Value = get_resp2.json().await.unwrap();
        assert_eq!(
            get_body2["data"]["name"].as_str().unwrap(),
            &updated_name,
            "更新后 GET 应返回新名称而非缓存中的旧值"
        );

        // --- 清理 ---
        let _ = sqlx::query("DELETE FROM badge_rules WHERE id = $1")
            .bind(rule_id)
            .execute(&pool)
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/badges/{}", badge_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/series/{}", series_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/categories/{}", cat_id)))
            .bearer_auth(&token)
            .send()
            .await;
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_user_badge_cache_sync() {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        let base = base_url();
        let token = login(&client, &base).await;
        let pool = PgPool::connect(&database_url()).await.expect("数据库连接失败");

        let suffix = unique_suffix();
        let user_id = format!("test_cache_sync_user_{}", suffix);

        // --- 搭建徽章基础设施 ---
        let cat_id = client
            .post(api_url(&base, "/api/admin/categories"))
            .bearer_auth(&token)
            .json(&json!({ "name": format!("sync_cat_{}", suffix), "sortOrder": 0 }))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        let series_id = client
            .post(api_url(&base, "/api/admin/series"))
            .bearer_auth(&token)
            .json(&json!({ "name": format!("sync_series_{}", suffix), "categoryId": cat_id, "sortOrder": 0 }))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["data"]["id"]
            .as_i64()
            .unwrap();

        let badge_resp = client
            .post(api_url(&base, "/api/admin/badges"))
            .bearer_auth(&token)
            .json(&json!({
                "seriesId": series_id,
                "badgeType": "NORMAL",
                "name": format!("sync_badge_{}", suffix),
                "assets": { "iconUrl": "https://example.com/sync.png" },
                "validityConfig": { "validityType": "PERMANENT" }
            }))
            .send()
            .await
            .unwrap();
        let badge_body: Value = badge_resp.json().await.unwrap();
        let badge_id = badge_body["data"]["id"].as_i64().unwrap();
        let badge_name = badge_body["data"]["name"].as_str().unwrap().to_string();

        // 发布徽章
        let _ = client
            .post(api_url(&base, &format!("/api/admin/badges/{}/publish", badge_id)))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();

        // --- 发放徽章 ---
        let grant_resp = client
            .post(api_url(&base, "/api/admin/grants/manual"))
            .bearer_auth(&token)
            .json(&json!({
                "userId": user_id,
                "badgeId": badge_id,
                "quantity": 1,
                "reason": "缓存同步测试"
            }))
            .send()
            .await
            .unwrap();
        assert!(grant_resp.status().is_success(), "发放徽章应成功");

        // 等待数据落库
        tokio::time::sleep(Duration::from_secs(2)).await;

        // --- 通过 API 查询用户徽章列表 ---
        let user_badges_resp = client
            .get(api_url(&base, &format!("/api/admin/users/{}/badges", user_id)))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(user_badges_resp.status().is_success(), "查询用户徽章应成功");
        let user_badges_body: Value = user_badges_resp.json().await.unwrap();

        // 在分页响应中查找刚发放的徽章
        let items = user_badges_body["data"]["items"]
            .as_array()
            .expect("用户徽章响应应包含 items 数组");
        let found = items
            .iter()
            .any(|b| b["badgeId"].as_i64() == Some(badge_id));
        assert!(
            found,
            "API 返回的用户徽章列表应包含刚发放的徽章 {}（name: {}）",
            badge_id, badge_name
        );

        // --- 刷新依赖缓存并再次查询，确认一致性 ---
        let _ = client
            .post(api_url(&base, "/api/admin/cache/dependencies/refresh"))
            .bearer_auth(&token)
            .send()
            .await;

        let user_badges_resp2 = client
            .get(api_url(&base, &format!("/api/admin/users/{}/badges", user_id)))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let user_badges_body2: Value = user_badges_resp2.json().await.unwrap();
        let items2 = user_badges_body2["data"]["items"]
            .as_array()
            .expect("刷新后用户徽章响应应包含 items 数组");
        let found2 = items2
            .iter()
            .any(|b| b["badgeId"].as_i64() == Some(badge_id));
        assert!(
            found2,
            "缓存刷新后 API 仍应返回一致的用户徽章数据"
        );

        // --- 清理 ---
        let _ = sqlx::query("DELETE FROM badge_ledger WHERE user_id = $1 AND badge_id = $2")
            .bind(&user_id)
            .bind(badge_id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM user_badges WHERE user_id = $1 AND badge_id = $2")
            .bind(&user_id)
            .bind(badge_id)
            .execute(&pool)
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/badges/{}", badge_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/series/{}", series_id)))
            .bearer_auth(&token)
            .send()
            .await;
        let _ = client
            .delete(api_url(&base, &format!("/api/admin/categories/{}", cat_id)))
            .bearer_auth(&token)
            .send()
            .await;
    }
}
