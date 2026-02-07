-- 回滚脚本：撤销规则模板表及 badge_rules 表扩展
-- 对应 UP 迁移：20250204_001_rule_templates.sql
-- 此脚本移除 badge_rules 表上的模板关联列，然后删除规则模板表
-- 注意：需先移除 badge_rules 的外键引用，再删除 rule_templates 表

-- ============================================
-- 1. 移除 badge_rules 表上的模板关联索引
-- ============================================
DROP INDEX IF EXISTS idx_badge_rules_template;

-- ============================================
-- 2. 移除 badge_rules 表上的模板关联列
-- ============================================
ALTER TABLE badge_rules DROP COLUMN IF EXISTS template_params;
ALTER TABLE badge_rules DROP COLUMN IF EXISTS template_version;
ALTER TABLE badge_rules DROP COLUMN IF EXISTS template_id;

-- ============================================
-- 3. 删除触发器
-- ============================================
DROP TRIGGER IF EXISTS update_rule_templates_updated_at ON rule_templates;

-- ============================================
-- 4. 删除规则模板表
-- ============================================
DROP TABLE IF EXISTS rule_templates CASCADE;
