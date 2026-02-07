-- 回滚脚本：撤销自动权益发放相关表
-- 对应 UP 迁移：20250207_001_auto_benefit.sql
-- 此脚本删除 auto_benefit_grants 和 auto_benefit_evaluation_logs 表及其索引和触发器
-- 注意：执行此回滚将丢失所有自动权益发放记录和评估日志数据

-- ============================================
-- 1. 删除触发器
-- ============================================
DROP TRIGGER IF EXISTS update_auto_benefit_grants_updated_at ON auto_benefit_grants;

-- ============================================
-- 2. 删除索引（DROP TABLE CASCADE 会自动删除，这里显式列出便于单独操作）
-- ============================================

-- auto_benefit_evaluation_logs 索引
DROP INDEX IF EXISTS idx_auto_benefit_eval_badge_time;
DROP INDEX IF EXISTS idx_auto_benefit_eval_time;
DROP INDEX IF EXISTS idx_auto_benefit_eval_badge;
DROP INDEX IF EXISTS idx_auto_benefit_eval_user;

-- auto_benefit_grants 索引
DROP INDEX IF EXISTS idx_auto_benefit_grants_benefit;
DROP INDEX IF EXISTS idx_auto_benefit_grants_created;
DROP INDEX IF EXISTS idx_auto_benefit_grants_status;
DROP INDEX IF EXISTS idx_auto_benefit_grants_trigger;
DROP INDEX IF EXISTS idx_auto_benefit_grants_user_rule;

-- ============================================
-- 3. 按创建反序删除表
-- ============================================
DROP TABLE IF EXISTS auto_benefit_evaluation_logs CASCADE;
DROP TABLE IF EXISTS auto_benefit_grants CASCADE;
