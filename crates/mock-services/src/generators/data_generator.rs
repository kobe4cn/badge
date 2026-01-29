//! 数据生成器
//!
//! 批量生成测试数据，用于填充模拟服务的内存存储。

use crate::models::{MockCoupon, MockOrder, MockUser};
use crate::store::MemoryStore;
use rand::Rng;
use std::ops::Range;

/// 数据生成器配置
///
/// 控制生成数据的数量和分布
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// 生成的用户数量
    pub user_count: usize,
    /// 每个用户的订单数量范围
    pub orders_per_user: Range<usize>,
    /// 每个用户的优惠券数量范围
    pub coupons_per_user: Range<usize>,
}

impl Default for GeneratorConfig {
    /// 默认配置：100 用户，每人 1-10 订单，0-5 优惠券
    fn default() -> Self {
        Self {
            user_count: 100,
            orders_per_user: 1..10,
            coupons_per_user: 0..5,
        }
    }
}

/// 批量数据生成器
///
/// 用于一次性生成大量测试数据并填充到内存存储中
pub struct DataGenerator {
    config: GeneratorConfig,
}

impl DataGenerator {
    /// 创建数据生成器
    pub fn new(config: GeneratorConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建生成器
    pub fn with_defaults() -> Self {
        Self::new(GeneratorConfig::default())
    }

    /// 生成指定数量的随机用户
    pub fn generate_users(&self) -> Vec<MockUser> {
        (0..self.config.user_count)
            .map(|_| MockUser::random())
            .collect()
    }

    /// 为指定用户生成随机订单
    ///
    /// 订单数量在配置的范围内随机
    pub fn generate_orders(&self, user_id: &str) -> Vec<MockOrder> {
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(self.config.orders_per_user.clone());

        (0..count).map(|_| MockOrder::random(user_id)).collect()
    }

    /// 为指定用户生成随机优惠券
    ///
    /// 优惠券数量在配置的范围内随机
    pub fn generate_coupons(&self, user_id: &str) -> Vec<MockCoupon> {
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(self.config.coupons_per_user.clone());

        (0..count).map(|_| MockCoupon::random(user_id)).collect()
    }

    /// 批量填充数据到存储
    ///
    /// 按照配置生成用户，并为每个用户生成订单和优惠券
    pub fn populate_stores(
        &self,
        users: &MemoryStore<MockUser>,
        orders: &MemoryStore<MockOrder>,
        coupons: &MemoryStore<MockCoupon>,
    ) {
        let generated_users = self.generate_users();

        for user in generated_users {
            let user_id = user.user_id.clone();

            // 存储用户
            users.insert(&user_id, user);

            // 生成并存储订单
            let user_orders = self.generate_orders(&user_id);
            for order in user_orders {
                let order_id = order.order_id.clone();
                orders.insert(&order_id, order);
            }

            // 生成并存储优惠券
            let user_coupons = self.generate_coupons(&user_id);
            for coupon in user_coupons {
                let coupon_id = coupon.coupon_id.clone();
                coupons.insert(&coupon_id, coupon);
            }
        }
    }

    /// 获取配置
    pub fn config(&self) -> &GeneratorConfig {
        &self.config
    }
}

/// 统计数据生成结果
#[derive(Debug, Clone)]
pub struct GenerationStats {
    pub users_count: usize,
    pub orders_count: usize,
    pub coupons_count: usize,
}

impl GenerationStats {
    /// 从存储中收集统计信息
    pub fn from_stores(
        users: &MemoryStore<MockUser>,
        orders: &MemoryStore<MockOrder>,
        coupons: &MemoryStore<MockCoupon>,
    ) -> Self {
        Self {
            users_count: users.count(),
            orders_count: orders.count(),
            coupons_count: coupons.count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_users() {
        let config = GeneratorConfig {
            user_count: 10,
            orders_per_user: 1..5,
            coupons_per_user: 0..3,
        };
        let generator = DataGenerator::new(config);

        let users = generator.generate_users();
        assert_eq!(users.len(), 10);

        // 确保所有用户 ID 唯一
        let unique_ids: std::collections::HashSet<_> =
            users.iter().map(|u| u.user_id.clone()).collect();
        assert_eq!(unique_ids.len(), 10);
    }

    #[test]
    fn test_generate_orders() {
        let config = GeneratorConfig {
            user_count: 1,
            orders_per_user: 3..6,
            coupons_per_user: 0..1,
        };
        let generator = DataGenerator::new(config);

        let orders = generator.generate_orders("test-user");

        // 订单数量在配置范围内
        assert!(orders.len() >= 3 && orders.len() < 6);
        // 所有订单属于同一用户
        assert!(orders.iter().all(|o| o.user_id == "test-user"));
    }

    #[test]
    fn test_generate_coupons() {
        let config = GeneratorConfig {
            user_count: 1,
            orders_per_user: 1..2,
            coupons_per_user: 2..5,
        };
        let generator = DataGenerator::new(config);

        let coupons = generator.generate_coupons("test-user");

        // 优惠券数量在配置范围内
        assert!(coupons.len() >= 2 && coupons.len() < 5);
        // 所有优惠券属于同一用户
        assert!(coupons.iter().all(|c| c.user_id == "test-user"));
    }

    #[test]
    fn test_populate_stores() {
        let config = GeneratorConfig {
            user_count: 5,
            orders_per_user: 2..4,
            coupons_per_user: 1..3,
        };
        let generator = DataGenerator::new(config);

        let users: MemoryStore<MockUser> = MemoryStore::new();
        let orders: MemoryStore<MockOrder> = MemoryStore::new();
        let coupons: MemoryStore<MockCoupon> = MemoryStore::new();

        generator.populate_stores(&users, &orders, &coupons);

        // 验证用户数量
        assert_eq!(users.count(), 5);
        // 验证订单和优惠券数量大于 0
        assert!(orders.count() > 0);
        assert!(coupons.count() > 0);

        // 验证统计
        let stats = GenerationStats::from_stores(&users, &orders, &coupons);
        assert_eq!(stats.users_count, 5);
        assert!(stats.orders_count >= 10); // 至少 5 用户 * 2 订单
        assert!(stats.coupons_count >= 5); // 至少 5 用户 * 1 优惠券
    }

    #[test]
    fn test_default_config() {
        let config = GeneratorConfig::default();
        assert_eq!(config.user_count, 100);
        assert_eq!(config.orders_per_user, 1..10);
        assert_eq!(config.coupons_per_user, 0..5);
    }
}
