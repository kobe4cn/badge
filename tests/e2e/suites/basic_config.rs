//! 基础配置测试套件
//!
//! 测试分类、系列、徽章的 CRUD 操作。

use crate::data::*;
use crate::helpers::*;
use crate::setup::TestEnvironment;

/// 分类管理测试
#[cfg(test)]
mod category_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_category() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建分类
        let req = TestCategories::achievement();
        let result = env.api.create_category(&req).await;

        assert!(result.is_ok(), "创建分类应该成功: {:?}", result.err());
        let category = result.unwrap();
        assert_eq!(category.name, req.name);
        assert_eq!(category.icon_url, req.icon_url);

        // 验证数据库
        let count = env
            .db
            .count("badge_categories", &format!("id = {}", category.id))
            .await
            .unwrap();
        assert_eq!(count, 1, "数据库应该有一条记录");

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_child_category() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建父分类（当前 API 不支持嵌套分类，仅验证分类创建）
        let _parent = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();

        // 创建另一个分类（当前 API 不支持嵌套分类）
        let child_req = CreateCategoryRequest::new("Test子分类")
            .with_description("测试子分类");
        let child = env.api.create_category(&child_req).await.unwrap();

        // 验证分类创建成功
        assert!(!child.name.is_empty(), "分类名称不应为空");

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_update_category() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建分类
        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();

        // 更新分类
        let update_req = UpdateCategoryRequest {
            name: Some("Test更新后的名称".to_string()),
            icon_url: None,
            sort_order: Some(10),
        };
        let updated = env
            .api
            .update_category(category.id, &update_req)
            .await
            .unwrap();

        assert_eq!(updated.name, "Test更新后的名称");

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_delete_empty_category() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建空分类
        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();

        // 删除分类
        let result = env.api.delete_category(category.id).await;
        assert!(result.is_ok(), "删除空分类应该成功");

        // 验证已删除
        let count = env
            .db
            .count("badge_categories", &format!("id = {}", category.id))
            .await
            .unwrap();
        assert_eq!(count, 0, "分类应该已删除");

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_delete_non_empty_category_fails() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建分类和系列
        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();
        let _series = env
            .api
            .create_series(&TestSeries::newcomer(category.id))
            .await
            .unwrap();

        // 尝试删除非空分类
        let result = env.api.delete_category(category.id).await;
        assert!(result.is_err(), "删除非空分类应该失败");

        env.cleanup().await.unwrap();
    }
}

/// 系列管理测试
#[cfg(test)]
mod series_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_series() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建分类
        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();

        // 创建系列
        let req = TestSeries::newcomer(category.id);
        let series = env.api.create_series(&req).await.unwrap();

        assert_eq!(series.name, req.name);
        assert_eq!(series.category_id, category.id);

        env.cleanup().await.unwrap();
    }
}

/// 徽章管理测试
#[cfg(test)]
mod badge_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_badge() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建分类和系列
        let category = env
            .api
            .create_category(&TestCategories::consumption())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&TestSeries::spending(category.id))
            .await
            .unwrap();

        // 创建徽章
        let req = TestBadges::first_purchase(series.id);
        let badge = env.api.create_badge(&req).await.unwrap();

        assert_eq!(badge.name, req.name);
        assert_eq!(badge.series_id, series.id);
        assert_eq!(badge.status, "DRAFT", "新徽章应该是草稿状态");

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_limited_badge() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let category = env
            .api
            .create_category(&TestCategories::event())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&TestSeries::newcomer(category.id))
            .await
            .unwrap();

        // 创建限量徽章
        let req = TestBadges::limited_edition(series.id, 100);
        let badge = env.api.create_badge(&req).await.unwrap();

        assert_eq!(badge.max_supply, Some(100));
        assert_eq!(badge.issued_count, 0);

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_badge_status_transition() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&TestSeries::newcomer(category.id))
            .await
            .unwrap();
        let badge = env
            .api
            .create_badge(&TestBadges::first_purchase(series.id))
            .await
            .unwrap();

        // 草稿 -> 上线 (publish)
        let updated = env
            .api
            .publish_badge(badge.id)
            .await
            .unwrap();
        assert_eq!(updated.status, "ACTIVE");

        // 上线 -> 下线 (offline)
        let updated = env
            .api
            .offline_badge(badge.id)
            .await
            .unwrap();
        assert_eq!(updated.status, "INACTIVE");

        env.cleanup().await.unwrap();
    }
}
