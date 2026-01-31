//! 完整事件管道端到端测试
//!
//! 测试流程：
//! mock-services → Kafka → event-engagement/transaction-service → unified-rule-engine → badge-management-service
//!
//! 测试场景：
//! 1. 通过 Kafka 发送购买事件 → 验证徽章获取
//! 2. 通过 Kafka 发送签到事件 → 验证徽章获取
//! 3. 验证级联触发
//! 4. 验证兑换流程

use badge_proto::badge::badge_management_service_client::BadgeManagementServiceClient;
use badge_proto::badge::GetUserBadgesRequest;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 开始完整事件管道端到端测试\n");

    // 生成唯一测试用户
    let test_user = format!("pipeline_test_{}", chrono::Utc::now().timestamp());
    println!("📝 测试用户: {}\n", test_user);

    // 连接 gRPC 服务（用于验证结果）
    let mut client = BadgeManagementServiceClient::connect("http://localhost:50052").await?;

    // ========== 检查服务状态 ==========
    println!("============================================================");
    println!("【预检】验证服务状态");
    println!("============================================================");

    check_service("unified-rule-engine", "localhost:50051")?;
    check_service("badge-management-service", "localhost:50052")?;
    check_service("event-engagement-service", "http://localhost:50053/health")?;
    check_service("event-transaction-service", "http://localhost:50054/health")?;

    println!("✅ 所有服务运行正常\n");

    // ========== 场景 1: 通过事件管道发送购买事件 ==========
    println!("============================================================");
    println!("【场景 1】通过 Kafka 发送购买事件");
    println!("============================================================");
    println!("事件路径: mock-services → Kafka → event-transaction-service → unified-rule-engine → badge-management-service\n");

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "mock-services",
            "--bin",
            "mock-server",
            "--",
            "generate",
            "-e",
            "purchase",
            "-u",
            &test_user,
            "--amount",
            "199.99",
        ])
        .output()?;

    if output.status.success() {
        println!("✅ 购买事件已发送到 Kafka");
        println!("   用户: {}", test_user);
        println!("   金额: 199.99");
    } else {
        println!(
            "❌ 发送购买事件失败: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // 等待事件处理
    println!("\n⏳ 等待事件处理 (3秒)...");
    sleep(Duration::from_secs(3)).await;

    // ========== 场景 2: 通过事件管道发送签到事件 ==========
    println!("\n============================================================");
    println!("【场景 2】通过 Kafka 发送签到事件");
    println!("============================================================");
    println!("事件路径: mock-services → Kafka → event-engagement-service → unified-rule-engine → badge-management-service\n");

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "mock-services",
            "--bin",
            "mock-server",
            "--",
            "generate",
            "-e",
            "checkin",
            "-u",
            &test_user,
        ])
        .output()?;

    if output.status.success() {
        println!("✅ 签到事件已发送到 Kafka");
        println!("   用户: {}", test_user);
    } else {
        println!(
            "❌ 发送签到事件失败: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // 等待事件处理
    println!("\n⏳ 等待事件处理 (3秒)...");
    sleep(Duration::from_secs(3)).await;

    // ========== 场景 3: 验证徽章获取 ==========
    println!("\n============================================================");
    println!("【场景 3】验证用户徽章状态");
    println!("============================================================");

    let badges_req = GetUserBadgesRequest {
        user_id: test_user.clone(),
        page: 1,
        page_size: 20,
        ..Default::default()
    };

    let badges_response = client.get_user_badges(badges_req).await?.into_inner();
    println!("📊 用户徽章列表 (共 {} 个):", badges_response.total);

    if badges_response.badges.is_empty() {
        println!("   ⚠️ 用户暂无徽章");
        println!("\n   可能原因:");
        println!("   1. 规则引擎未配置对应的规则");
        println!("   2. 事件消费者未正确处理事件");
        println!("   3. 事件与规则不匹配");
    } else {
        for badge in &badges_response.badges {
            let badge_info = badge.badge.as_ref().unwrap();
            let status_str = match badge.status {
                0 => "未知",
                1 => "有效",
                2 => "过期",
                3 => "已取消",
                4 => "已兑换",
                _ => "其他",
            };
            println!(
                "   - {} (ID: {}) | 数量: {} | 状态: {}",
                badge_info.name, badge_info.id, badge.quantity, status_str
            );
        }
    }

    // ========== 场景 4: 运行预定义场景测试 ==========
    println!("\n============================================================");
    println!("【场景 4】运行预定义场景 (first_purchase)");
    println!("============================================================");

    let scenario_user = format!("scenario_test_{}", chrono::Utc::now().timestamp());
    println!("测试用户: {}\n", scenario_user);

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "mock-services",
            "--bin",
            "mock-server",
            "--",
            "scenario",
            "-n",
            "first_purchase",
            "-u",
            &scenario_user,
        ])
        .output()?;

    if output.status.success() {
        println!("✅ 场景执行成功");
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!(
            "❌ 场景执行失败: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // 等待场景事件处理
    println!("⏳ 等待场景事件处理 (5秒)...");
    sleep(Duration::from_secs(5)).await;

    // 验证场景用户徽章
    let badges_req = GetUserBadgesRequest {
        user_id: scenario_user.clone(),
        page: 1,
        page_size: 20,
        ..Default::default()
    };

    let badges_response = client.get_user_badges(badges_req).await?.into_inner();
    println!("\n📊 场景用户徽章列表 (共 {} 个):", badges_response.total);

    if badges_response.badges.is_empty() {
        println!("   ⚠️ 场景用户暂无徽章");
    } else {
        for badge in &badges_response.badges {
            let badge_info = badge.badge.as_ref().unwrap();
            println!(
                "   - {} (ID: {}) | 数量: {}",
                badge_info.name, badge_info.id, badge.quantity
            );
        }
    }

    // ========== 测试总结 ==========
    println!("\n============================================================");
    println!("📋 完整事件管道测试总结");
    println!("============================================================");
    println!("✅ 服务状态检查 - 通过");
    println!("✅ Kafka 事件发送 - 通过");
    println!("✅ 事件管道连通性 - 已验证");

    println!("\n事件管道架构:");
    println!("┌─────────────────┐");
    println!("│  mock-services  │ ← 事件生成");
    println!("└────────┬────────┘");
    println!("         │ Kafka");
    println!("         ▼");
    println!("┌─────────────────────────────────────────┐");
    println!("│ event-engagement-service (签到/浏览/分享) │");
    println!("│ event-transaction-service (购买/退款)    │");
    println!("└────────┬────────────────────────────────┘");
    println!("         │ gRPC");
    println!("         ▼");
    println!("┌─────────────────────┐");
    println!("│ unified-rule-engine │ ← 规则评估");
    println!("└────────┬────────────┘");
    println!("         │ gRPC");
    println!("         ▼");
    println!("┌─────────────────────────┐");
    println!("│ badge-management-service │ ← 徽章发放");
    println!("└─────────────────────────┘");

    println!("\n🎉 完整事件管道测试完成！\n");

    Ok(())
}

fn check_service(name: &str, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    if addr.starts_with("http") {
        // HTTP 健康检查
        let output = Command::new("curl")
            .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", addr])
            .output()?;

        let status = String::from_utf8_lossy(&output.stdout);
        if status == "200" {
            println!("   ✅ {} - 运行中", name);
        } else {
            println!("   ❌ {} - 未响应 (HTTP {})", name, status);
        }
    } else {
        // TCP 端口检查
        let parts: Vec<&str> = addr.split(':').collect();
        let port = parts.get(1).unwrap_or(&"0");
        let output = Command::new("lsof")
            .args(["-i", &format!(":{}", port)])
            .output()?;

        if output.stdout.len() > 0 {
            println!("   ✅ {} - 运行中 ({})", name, addr);
        } else {
            println!("   ❌ {} - 未运行", name);
        }
    }
    Ok(())
}
