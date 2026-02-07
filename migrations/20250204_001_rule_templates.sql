-- 规则模板表
-- 支持规则引擎的模板参数化功能，允许通过预定义模板快速创建规则

-- ==================== 规则模板 ====================

CREATE TABLE IF NOT EXISTS rule_templates (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,       -- 模板代码，如 'purchase_gte'
    name VARCHAR(100) NOT NULL,
    description TEXT,
    category VARCHAR(50) NOT NULL,          -- basic, advanced, industry
    subcategory VARCHAR(50),                -- e-commerce, gaming, o2o
    template_json JSONB NOT NULL,           -- 带 ${param} 占位符的规则模板
    parameters JSONB NOT NULL DEFAULT '[]', -- 参数定义数组
    version VARCHAR(20) NOT NULL DEFAULT '1.0',
    is_system BOOLEAN NOT NULL DEFAULT FALSE, -- 系统内置模板不可删除
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE rule_templates IS '规则模板，定义可复用的规则模板，支持参数化配置';
COMMENT ON COLUMN rule_templates.code IS '模板唯一代码，如 purchase_gte、login_streak 等';
COMMENT ON COLUMN rule_templates.category IS '模板分类：basic-基础模板，advanced-高级模板，industry-行业模板';
COMMENT ON COLUMN rule_templates.subcategory IS '子分类：e-commerce-电商，gaming-游戏，o2o-线下服务等';
COMMENT ON COLUMN rule_templates.template_json IS '规则模板JSON，使用 ${param} 占位符表示可配置参数';
COMMENT ON COLUMN rule_templates.parameters IS '参数定义数组，格式：[{"name": "amount", "type": "number", "required": true, "default": 100}]';
COMMENT ON COLUMN rule_templates.version IS '模板版本号，用于追踪模板变更';
COMMENT ON COLUMN rule_templates.is_system IS '是否为系统内置模板，内置模板不允许删除';

-- 索引
CREATE INDEX IF NOT EXISTS idx_rule_templates_category ON rule_templates(category, subcategory);
CREATE INDEX IF NOT EXISTS idx_rule_templates_code ON rule_templates(code);
CREATE INDEX IF NOT EXISTS idx_rule_templates_enabled ON rule_templates(enabled) WHERE enabled = TRUE;

-- 触发器：自动更新 updated_at
DROP TRIGGER IF EXISTS update_rule_templates_updated_at ON rule_templates;
CREATE TRIGGER update_rule_templates_updated_at
    BEFORE UPDATE ON rule_templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ==================== 扩展 badge_rules 表 ====================

-- 添加模板关联字段
ALTER TABLE badge_rules ADD COLUMN IF NOT EXISTS template_id BIGINT REFERENCES rule_templates(id);
ALTER TABLE badge_rules ADD COLUMN IF NOT EXISTS template_version VARCHAR(20);
ALTER TABLE badge_rules ADD COLUMN IF NOT EXISTS template_params JSONB DEFAULT '{}';

COMMENT ON COLUMN badge_rules.template_id IS '关联的规则模板ID，NULL表示自定义规则';
COMMENT ON COLUMN badge_rules.template_version IS '创建时使用的模板版本，用于检测模板是否已更新';
COMMENT ON COLUMN badge_rules.template_params IS '模板参数值，JSON格式，如 {"amount": 500, "days": 30}';

-- 索引：加速模板关联查询
CREATE INDEX IF NOT EXISTS idx_badge_rules_template ON badge_rules(template_id) WHERE template_id IS NOT NULL;
