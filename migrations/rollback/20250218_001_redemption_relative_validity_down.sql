-- 回滚 20250218_001_redemption_relative_validity
ALTER TABLE badge_redemption_rules DROP COLUMN IF EXISTS relative_days;
ALTER TABLE badge_redemption_rules DROP COLUMN IF EXISTS validity_type;
