-- 回滚 20250212_001_rule_name_column
ALTER TABLE badge_rules DROP COLUMN IF EXISTS description;
ALTER TABLE badge_rules DROP COLUMN IF EXISTS name;
