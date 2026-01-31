-- 测试数据初始化脚本
-- 用于本地开发和 E2E 测试环境

-- ============================================
-- 1. 清理旧数据（可选，谨慎使用）
-- ============================================
-- TRUNCATE badges, badge_series, badge_categories, badge_rules,
--          user_badges, badge_ledger, badge_dependencies,
--          benefits, badge_redemption_rules, redemption_orders CASCADE;

-- ============================================
-- 2. 基础分类和系列
-- ============================================
INSERT INTO badge_categories (id, name, icon_url, sort_order, status) VALUES
  (1, '新手任务', 'https://cdn.example.com/cat/newbie.png', 1, 'ACTIVE'),
  (2, '消费成就', 'https://cdn.example.com/cat/purchase.png', 2, 'ACTIVE'),
  (3, '社交互动', 'https://cdn.example.com/cat/social.png', 3, 'ACTIVE'),
  (4, '限定活动', 'https://cdn.example.com/cat/event.png', 4, 'ACTIVE')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;

INSERT INTO badge_series (id, category_id, name, description, sort_order, status) VALUES
  (1, 1, '入门系列', '完成基础任务获得', 1, 'ACTIVE'),
  (2, 2, '购物系列', '消费达成获得', 1, 'ACTIVE'),
  (3, 3, '社交系列', '互动分享获得', 1, 'ACTIVE'),
  (4, 4, '活动系列', '限时活动获得', 1, 'ACTIVE')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;

-- ============================================
-- 3. 徽章定义
-- ============================================
INSERT INTO badges (id, series_id, badge_type, name, description, obtain_description, assets, validity_config, status) VALUES
  -- 新手任务
  (1, 1, 'NORMAL', '新用户徽章', '欢迎加入', '注册即获得',
   '{"iconUrl": "https://cdn.example.com/badge/newbie.png"}',
   '{"validityType": "PERMANENT"}', 'ACTIVE'),
  (2, 1, 'NORMAL', '首次签到', '完成首次签到', '签到一次即可获得',
   '{"iconUrl": "https://cdn.example.com/badge/checkin.png"}',
   '{"validityType": "PERMANENT"}', 'ACTIVE'),

  -- 购物系列
  (3, 2, 'NORMAL', '首次购买', '完成首次购买', '购买任意商品即可获得',
   '{"iconUrl": "https://cdn.example.com/badge/first-buy.png"}',
   '{"validityType": "PERMANENT"}', 'ACTIVE'),
  (4, 2, 'NORMAL', '购物新星', '购买金额满100元', '单笔购买满100元',
   '{"iconUrl": "https://cdn.example.com/badge/shopping-star.png"}',
   '{"validityType": "PERMANENT"}', 'ACTIVE'),
  (5, 2, 'ACHIEVEMENT', '购物达人', '累计购买满1000元', '累计消费满1000元',
   '{"iconUrl": "https://cdn.example.com/badge/shopping-master.png"}',
   '{"validityType": "PERMANENT"}', 'ACTIVE'),

  -- 社交系列
  (6, 3, 'NORMAL', '社交达人', '首次分享', '分享到社交平台',
   '{"iconUrl": "https://cdn.example.com/badge/social.png"}',
   '{"validityType": "PERMANENT"}', 'ACTIVE'),
  (7, 3, 'NORMAL', '评价达人', '发表首条评价', '评价任意商品',
   '{"iconUrl": "https://cdn.example.com/badge/review.png"}',
   '{"validityType": "PERMANENT"}', 'ACTIVE'),

  -- 成就徽章（级联触发）
  (8, 3, 'ACHIEVEMENT', '互动KOC', '签到+社交双达成', '同时拥有首次签到和社交达人徽章',
   '{"iconUrl": "https://cdn.example.com/badge/koc.png"}',
   '{"validityType": "PERMANENT"}', 'ACTIVE'),

  -- 限定兑换徽章
  (9, 4, 'LIMITED', '乐园新星', '兑换限定徽章', '使用互动KOC+首次购买兑换',
   '{"iconUrl": "https://cdn.example.com/badge/park-star.png"}',
   '{"validityType": "RELATIVE_DAYS", "relativeDays": 365}', 'ACTIVE')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, status = EXCLUDED.status;

-- 重置序列
SELECT setval('badges_id_seq', (SELECT MAX(id) FROM badges));
SELECT setval('badge_series_id_seq', (SELECT MAX(id) FROM badge_series));
SELECT setval('badge_categories_id_seq', (SELECT MAX(id) FROM badge_categories));

-- ============================================
-- 4. 规则配置
-- ============================================
INSERT INTO badge_rules (id, code, name, badge_id, event_type, rule_json, service_group, enabled) VALUES
  -- 签到规则
  (1, 'first_checkin', '首次签到规则', 2, 'checkin',
   '{"root": {"type": "condition", "field": "event.type", "operator": "eq", "value": "checkin"}}',
   'engagement', true),

  -- 购买规则
  (2, 'first_purchase', '首次购买规则', 3, 'purchase',
   '{"root": {"type": "condition", "field": "event.type", "operator": "eq", "value": "purchase"}}',
   'transaction', true),
  (3, 'purchase_100', '购物新星规则', 4, 'purchase',
   '{"root": {"type": "logical_group", "operator": "and", "children": [
     {"type": "condition", "field": "event.type", "operator": "eq", "value": "purchase"},
     {"type": "condition", "field": "order.amount", "operator": "gte", "value": 100}
   ]}}',
   'transaction', true),

  -- 社交规则
  (4, 'first_share', '社交达人规则', 6, 'share',
   '{"root": {"type": "condition", "field": "event.type", "operator": "eq", "value": "share"}}',
   'engagement', true),
  (5, 'first_review', '评价达人规则', 7, 'review',
   '{"root": {"type": "condition", "field": "event.type", "operator": "eq", "value": "review"}}',
   'engagement', true)
ON CONFLICT (id) DO UPDATE SET enabled = EXCLUDED.enabled;

SELECT setval('badge_rules_id_seq', (SELECT MAX(id) FROM badge_rules));

-- ============================================
-- 5. 级联依赖配置
-- ============================================
-- 互动KOC = 首次签到 + 社交达人（AND 关系，自动触发）
INSERT INTO badge_dependencies (badge_id, depends_on_badge_id, dependency_type, required_quantity, auto_trigger, priority, dependency_group_id, enabled) VALUES
  (8, 2, 'prerequisite', 1, true, 1, 'koc_prereqs', true),  -- 依赖首次签到
  (8, 6, 'prerequisite', 1, true, 2, 'koc_prereqs', true)   -- 依赖社交达人
ON CONFLICT DO NOTHING;

-- 乐园新星 = 消耗(互动KOC + 首次购买)（手动兑换）
INSERT INTO badge_dependencies (badge_id, depends_on_badge_id, dependency_type, required_quantity, auto_trigger, priority, dependency_group_id, enabled) VALUES
  (9, 8, 'consume', 1, false, 1, 'park_star_consume', true),  -- 消耗互动KOC
  (9, 3, 'consume', 1, false, 2, 'park_star_consume', true)   -- 消耗首次购买
ON CONFLICT DO NOTHING;

-- ============================================
-- 6. 权益配置
-- ============================================
INSERT INTO benefits (id, code, name, description, benefit_type, external_id, external_system, total_stock, remaining_stock, redeemed_count, enabled) VALUES
  (1, 'PARK_TICKET_COUPON', '乐园门票优惠券', '兑换乐园新星徽章获得的门票优惠券',
   'COUPON', 'coupon-park-2024', 'coupon-platform', 10000, 10000, 0, true),
  (2, 'VIP_UPGRADE_VOUCHER', 'VIP升级券', '可用于 VIP 等级提升',
   'COUPON', 'vip-upgrade-2024', 'membership-platform', 5000, 5000, 0, true)
ON CONFLICT (id) DO UPDATE SET remaining_stock = EXCLUDED.remaining_stock;

SELECT setval('benefits_id_seq', (SELECT MAX(id) FROM benefits));

-- ============================================
-- 7. 兑换规则配置
-- ============================================
INSERT INTO badge_redemption_rules (id, name, description, benefit_id, required_badges, frequency_config, enabled) VALUES
  (1, '乐园新星兑换规则', '使用互动KOC和首次购买徽章兑换乐园新星徽章',
   1,
   '[{"badgeId": 8, "quantity": 1}, {"badgeId": 3, "quantity": 1}]',
   '{"maxPerUser": 1}',
   true)
ON CONFLICT (id) DO UPDATE SET enabled = EXCLUDED.enabled;

SELECT setval('badge_redemption_rules_id_seq', (SELECT MAX(id) FROM badge_redemption_rules));

-- ============================================
-- 8. 验证数据
-- ============================================
DO $$
BEGIN
  RAISE NOTICE '=== 测试数据初始化完成 ===';
  RAISE NOTICE '徽章数量: %', (SELECT COUNT(*) FROM badges WHERE status = 'ACTIVE');
  RAISE NOTICE '规则数量: %', (SELECT COUNT(*) FROM badge_rules WHERE enabled = true);
  RAISE NOTICE '级联依赖: %', (SELECT COUNT(*) FROM badge_dependencies WHERE enabled = true);
  RAISE NOTICE '兑换规则: %', (SELECT COUNT(*) FROM badge_redemption_rules WHERE enabled = true);
END $$;
