//! 数据库性能测试
//!
//! 测试数据库在高并发读写下的性能表现。

use super::super::{LoadTestConfig, LoadTestRunner, PerformanceAssertions};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
mod database_tests {
    use super::*;

    /// 获取数据库连接池
    async fn get_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://badge:badge@localhost:5432/badge_test".to_string());
        PgPool::connect(&database_url)
            .await
            .expect("无法连接数据库")
    }

    /// 用户徽章查询性能测试
    #[tokio::test]
    #[ignore = "需要数据库"]
    async fn test_user_badge_query_performance() {
        let pool = get_pool().await;
        let pool = Arc::new(pool);

        let config = LoadTestConfig {
            concurrent_users: 100,
            duration: Duration::from_secs(30),
            requests_per_second: Some(1000),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(5),
        };

        let runner = LoadTestRunner::new(config.clone());

        let metrics = runner
            .run(move || {
                let pool = pool.clone();
                async move {
                    let user_id = format!("user_{}", rand::random::<u32>() % 10000);
                    let start = Instant::now();

                    // 使用原生 SQL 避免编译时检查
                    let result: Result<Vec<(i64, String, Option<String>)>, sqlx::Error> = sqlx::query_as(
                        r#"
                        SELECT ub.id, b.name as badge_name, b.assets->>'iconUrl' as icon_url
                        FROM user_badges ub
                        JOIN badges b ON ub.badge_id = b.id
                        WHERE ub.user_id = $1
                        AND ub.status = 'active'
                        ORDER BY ub.first_acquired_at DESC
                        LIMIT 50
                        "#,
                    )
                    .bind(&user_id)
                    .fetch_all(pool.as_ref())
                    .await;

                    let latency = start.elapsed();

                    match result {
                        Ok(_) => Ok(latency),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        // 数据库查询目标: P99 <= 50ms
        PerformanceAssertions::assert_success_rate(&metrics, 99.9);
        PerformanceAssertions::assert_p99_latency(&metrics, 50.0);
    }

    /// 徽章发放写入性能测试
    #[tokio::test]
    #[ignore = "需要数据库"]
    async fn test_badge_grant_write_performance() {
        let pool = get_pool().await;
        let pool = Arc::new(pool);

        let config = LoadTestConfig {
            concurrent_users: 50,
            duration: Duration::from_secs(30),
            requests_per_second: Some(500),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let metrics = runner
            .run(move || {
                let pool = pool.clone();
                let cnt = counter.clone();
                let seq = cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                async move {
                    let user_id = format!("perf_write_user_{}", seq);
                    let badge_id = (seq % 10 + 1) as i64;
                    let source_ref = format!("seq_{}", seq);
                    let start = Instant::now();

                    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

                    // 插入用户徽章
                    let result: Result<sqlx::postgres::PgQueryResult, sqlx::Error> = sqlx::query(
                        r#"
                        INSERT INTO user_badges (user_id, badge_id, status, source_type, source_ref)
                        VALUES ($1, $2, 'active', 'manual', $3)
                        ON CONFLICT (user_id, badge_id) DO NOTHING
                        "#,
                    )
                    .bind(&user_id)
                    .bind(badge_id)
                    .bind(&source_ref)
                    .execute(&mut *tx)
                    .await;

                    if result.is_err() {
                        tx.rollback().await.ok();
                        return Err(result.unwrap_err().to_string());
                    }

                    // 插入日志
                    let _ = sqlx::query(
                        r#"
                        INSERT INTO user_badge_logs (user_id, badge_id, action, source_type, source_ref_id)
                        VALUES ($1, $2, 'grant', 'manual', $3)
                        "#,
                    )
                    .bind(&user_id)
                    .bind(badge_id)
                    .bind(&source_ref)
                    .execute(&mut *tx)
                    .await;

                    tx.commit().await.map_err(|e| e.to_string())?;
                    let latency = start.elapsed();

                    Ok(latency)
                }
            })
            .await;

        // 写入性能目标: 成功率 >= 99%, P99 <= 100ms
        PerformanceAssertions::assert_success_rate(&metrics, 99.0);
        PerformanceAssertions::assert_p99_latency(&metrics, 100.0);
    }

    /// 并发更新性能测试 - 测试行级锁
    #[tokio::test]
    #[ignore = "需要数据库"]
    async fn test_concurrent_update_with_lock() {
        let pool = get_pool().await;
        let pool = Arc::new(pool);

        let config = LoadTestConfig {
            concurrent_users: 20,
            duration: Duration::from_secs(30),
            requests_per_second: Some(100),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        };

        let runner = LoadTestRunner::new(config.clone());
        // 固定徽章 ID 以测试锁竞争
        let target_badge_id = 1i64;

        let metrics = runner
            .run(move || {
                let pool = pool.clone();

                async move {
                    let start = Instant::now();

                    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

                    // FOR UPDATE 锁定行
                    let result: Result<(i64, i64, Option<i64>), _> = sqlx::query_as(
                        r#"
                        SELECT id, issued_count, max_supply
                        FROM badges
                        WHERE id = $1
                        FOR UPDATE
                        "#,
                    )
                    .bind(target_badge_id)
                    .fetch_one(&mut *tx)
                    .await;

                    if result.is_err() {
                        tx.rollback().await.ok();
                        return Err(result.unwrap_err().to_string());
                    }

                    let (_, issued_count, max_supply) = result.unwrap();

                    // 检查库存并更新
                    if max_supply.is_none() || issued_count < max_supply.unwrap() {
                        let _ = sqlx::query(
                            r#"
                            UPDATE badges
                            SET issued_count = issued_count + 1,
                                updated_at = NOW()
                            WHERE id = $1
                            "#,
                        )
                        .bind(target_badge_id)
                        .execute(&mut *tx)
                        .await;
                    }

                    tx.commit().await.map_err(|e| e.to_string())?;
                    let latency = start.elapsed();

                    Ok(latency)
                }
            })
            .await;

        // 锁竞争场景容许较高延迟
        PerformanceAssertions::assert_success_rate(&metrics, 98.0);
        PerformanceAssertions::assert_p99_latency(&metrics, 500.0);
    }

    /// 批量查询性能测试
    #[tokio::test]
    #[ignore = "需要数据库"]
    async fn test_batch_query_performance() {
        let pool = get_pool().await;
        let pool = Arc::new(pool);

        let config = LoadTestConfig {
            concurrent_users: 30,
            duration: Duration::from_secs(30),
            requests_per_second: Some(100),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());

        let metrics = runner
            .run(move || {
                let pool = pool.clone();

                async move {
                    let start = Instant::now();

                    // 批量查询徽章统计
                    let result: Result<Vec<_>, _> = sqlx::query_as::<_, (i64, String, i64, i64)>(
                        r#"
                        SELECT
                            b.id,
                            b.name,
                            COUNT(ub.id) as grant_count,
                            COUNT(DISTINCT ub.user_id) as unique_users
                        FROM badges b
                        LEFT JOIN user_badges ub ON b.id = ub.badge_id AND ub.status = 'active'
                        WHERE b.status = 'active'
                        GROUP BY b.id, b.name
                        ORDER BY grant_count DESC
                        LIMIT 100
                        "#,
                    )
                    .fetch_all(pool.as_ref())
                    .await;

                    let latency = start.elapsed();

                    match result {
                        Ok(_) => Ok(latency),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        // 聚合查询容许较高延迟
        PerformanceAssertions::assert_success_rate(&metrics, 99.0);
        PerformanceAssertions::assert_p99_latency(&metrics, 200.0);
    }

    /// 索引效率测试
    #[tokio::test]
    #[ignore = "需要数据库"]
    async fn test_index_efficiency() {
        let pool = get_pool().await;
        let pool = Arc::new(pool);

        let config = LoadTestConfig {
            concurrent_users: 50,
            duration: Duration::from_secs(30),
            requests_per_second: Some(500),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(5),
        };

        let runner = LoadTestRunner::new(config.clone());

        let metrics = runner
            .run(move || {
                let pool = pool.clone();

                async move {
                    let start = Instant::now();

                    // 使用索引的时间范围查询
                    let result: Result<(i64,), sqlx::Error> = sqlx::query_as(
                        r#"
                        SELECT COUNT(*) as count
                        FROM user_badges
                        WHERE first_acquired_at >= NOW() - INTERVAL '7 days'
                        AND status = 'active'
                        "#,
                    )
                    .fetch_one(pool.as_ref())
                    .await;

                    let latency = start.elapsed();

                    match result {
                        Ok(_) => Ok(latency),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        // 索引查询应该很快
        PerformanceAssertions::assert_success_rate(&metrics, 99.9);
        PerformanceAssertions::assert_p99_latency(&metrics, 30.0);
    }
}
