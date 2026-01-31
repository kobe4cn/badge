-- 数据库 schema 与代码模型对齐
-- 补齐 redemption 相关表缺失的列

-- ============================================
-- 1. redemption_orders 表补充列
-- ============================================

-- 权益发放结果（JSON 格式，存储优惠券码等信息）
ALTER TABLE redemption_orders ADD COLUMN IF NOT EXISTS benefit_result JSONB;
COMMENT ON COLUMN redemption_orders.benefit_result IS '权益发放结果，如优惠券码、资产ID等';

-- 幂等键（防止重复兑换）
ALTER TABLE redemption_orders ADD COLUMN IF NOT EXISTS idempotency_key VARCHAR(100);
COMMENT ON COLUMN redemption_orders.idempotency_key IS '幂等键，防止重复提交';

CREATE UNIQUE INDEX IF NOT EXISTS idx_redemption_orders_idempotency
    ON redemption_orders(idempotency_key) WHERE idempotency_key IS NOT NULL;

-- ============================================
-- 2. badge_redemption_rules 表补充列
-- ============================================

-- 频率配置（JSON 格式，替代 frequency_type + frequency_limit）
ALTER TABLE badge_redemption_rules ADD COLUMN IF NOT EXISTS frequency_config JSONB NOT NULL DEFAULT '{}';
COMMENT ON COLUMN badge_redemption_rules.frequency_config IS '频率限制配置，如 {"maxPerUser": 5, "maxPerDay": 1}';

-- 规则生效时间（与 redeem_time_start/end 语义相同，代码使用这个命名）
ALTER TABLE badge_redemption_rules ADD COLUMN IF NOT EXISTS start_time TIMESTAMPTZ;
ALTER TABLE badge_redemption_rules ADD COLUMN IF NOT EXISTS end_time TIMESTAMPTZ;
COMMENT ON COLUMN badge_redemption_rules.start_time IS '规则生效开始时间';
COMMENT ON COLUMN badge_redemption_rules.end_time IS '规则生效结束时间';

-- 启用状态
ALTER TABLE badge_redemption_rules ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT TRUE;
COMMENT ON COLUMN badge_redemption_rules.enabled IS '规则是否启用';

-- 迁移旧数据：将 redeem_time_start/end 复制到 start_time/end_time
UPDATE badge_redemption_rules
SET start_time = redeem_time_start, end_time = redeem_time_end
WHERE start_time IS NULL AND redeem_time_start IS NOT NULL;

-- 迁移旧数据：根据 status 设置 enabled
UPDATE badge_redemption_rules SET enabled = (status = 'active');

-- ============================================
-- 3. benefits 表补充列
-- ============================================

-- 图标 URL
ALTER TABLE benefits ADD COLUMN IF NOT EXISTS icon_url VARCHAR(500);
COMMENT ON COLUMN benefits.icon_url IS '权益图标 URL';

-- 已兑换数量
ALTER TABLE benefits ADD COLUMN IF NOT EXISTS redeemed_count BIGINT NOT NULL DEFAULT 0;
COMMENT ON COLUMN benefits.redeemed_count IS '已兑换数量';

-- 启用状态
ALTER TABLE benefits ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT TRUE;
COMMENT ON COLUMN benefits.enabled IS '权益是否启用';

-- 迁移旧数据：根据 status 设置 enabled
UPDATE benefits SET enabled = (status = 'active');

-- 修改库存字段类型为 BIGINT（与代码模型一致）
ALTER TABLE benefits ALTER COLUMN total_stock TYPE BIGINT;
ALTER TABLE benefits ALTER COLUMN remaining_stock TYPE BIGINT;

-- ============================================
-- 4. 创建索引
-- ============================================

CREATE INDEX IF NOT EXISTS idx_redemption_rules_enabled ON badge_redemption_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_benefits_enabled ON benefits(enabled);
