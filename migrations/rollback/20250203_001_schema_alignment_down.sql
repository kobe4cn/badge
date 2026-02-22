-- 回滚 20250203_001_schema_alignment
DROP INDEX IF EXISTS idx_benefits_enabled;
ALTER TABLE benefits DROP COLUMN IF EXISTS enabled;
ALTER TABLE benefits DROP COLUMN IF EXISTS redeemed_count;
ALTER TABLE benefits DROP COLUMN IF EXISTS icon_url;
-- 注意：ALTER COLUMN TYPE 缩回可能因已有数据导致精度丢失
ALTER TABLE benefits ALTER COLUMN remaining_stock TYPE INT;
ALTER TABLE benefits ALTER COLUMN total_stock TYPE INT;

DROP INDEX IF EXISTS idx_redemption_rules_enabled;
ALTER TABLE badge_redemption_rules DROP COLUMN IF EXISTS enabled;
ALTER TABLE badge_redemption_rules DROP COLUMN IF EXISTS end_time;
ALTER TABLE badge_redemption_rules DROP COLUMN IF EXISTS start_time;
ALTER TABLE badge_redemption_rules DROP COLUMN IF EXISTS frequency_config;

DROP INDEX IF EXISTS idx_redemption_orders_idempotency;
ALTER TABLE redemption_orders DROP COLUMN IF EXISTS idempotency_key;
ALTER TABLE redemption_orders DROP COLUMN IF EXISTS benefit_result;
