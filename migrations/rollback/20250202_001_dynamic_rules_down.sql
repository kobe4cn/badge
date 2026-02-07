-- 回滚脚本：撤销动态规则加载支持
-- 对应 UP 迁移：20250202_001_dynamic_rules.sql
-- 此脚本删除事件类型配置表，并移除 badge_rules 表上新增的列和约束
-- 注意：需先移除 badge_rules 上的外键约束和列，再删除 event_types 表

-- ============================================
-- 1. 删除 badge_rules 表上新增的索引
-- ============================================
DROP INDEX IF EXISTS idx_badge_rules_event_type_enabled;

-- ============================================
-- 2. 移除 badge_rules 表上新增的约束
-- ============================================

-- 移除 rule_code 唯一约束
ALTER TABLE badge_rules DROP CONSTRAINT IF EXISTS uq_badge_rules_rule_code;

-- 移除 event_type 外键约束（必须在删除 event_types 表之前）
ALTER TABLE badge_rules DROP CONSTRAINT IF EXISTS fk_badge_rules_event_type;

-- ============================================
-- 3. 移除 badge_rules 表上新增的列
-- ============================================
ALTER TABLE badge_rules DROP COLUMN IF EXISTS global_granted;
ALTER TABLE badge_rules DROP COLUMN IF EXISTS global_quota;
ALTER TABLE badge_rules DROP COLUMN IF EXISTS rule_code;
ALTER TABLE badge_rules DROP COLUMN IF EXISTS event_type;

-- ============================================
-- 4. 删除触发器
-- ============================================
DROP TRIGGER IF EXISTS update_event_types_updated_at ON event_types;

-- ============================================
-- 5. 删除事件类型配置表
-- ============================================
DROP TABLE IF EXISTS event_types CASCADE;
