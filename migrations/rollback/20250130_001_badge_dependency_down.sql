-- 回滚脚本：撤销徽章依赖关系表
-- 对应 UP 迁移：20250130_001_badge_dependency.sql
-- 此脚本删除徽章依赖关系表及其触发器和索引

-- ============================================
-- 1. 删除触发器
-- ============================================
DROP TRIGGER IF EXISTS update_badge_dependencies_updated_at ON badge_dependencies;

-- ============================================
-- 2. 删除表（CASCADE 会自动清理索引和约束）
-- ============================================
DROP TABLE IF EXISTS badge_dependencies CASCADE;
