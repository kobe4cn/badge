-- 为 badge_rules 表添加 name 和 description 列
-- 支持规则命名和描述功能，方便管理后台展示和管理

-- 添加 name 列（规则显示名称）
DO $$ BEGIN
    ALTER TABLE badge_rules ADD COLUMN name VARCHAR(200);
EXCEPTION WHEN duplicate_column THEN NULL;
END $$;

-- 添加 description 列（规则描述）
DO $$ BEGIN
    ALTER TABLE badge_rules ADD COLUMN description TEXT;
EXCEPTION WHEN duplicate_column THEN NULL;
END $$;

COMMENT ON COLUMN badge_rules.name IS '规则显示名称，用于管理后台展示';
COMMENT ON COLUMN badge_rules.description IS '规则描述，说明规则用途和触发条件';

-- 为已有规则填充默认名称（基于 rule_code，若无则使用 id）
UPDATE badge_rules
SET name = COALESCE(rule_code, 'rule-' || id::TEXT)
WHERE name IS NULL;
