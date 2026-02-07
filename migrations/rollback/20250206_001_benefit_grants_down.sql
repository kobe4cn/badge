-- 回滚脚本：撤销权益发放记录表
-- 对应 UP 迁移：20250206_001_benefit_grants.sql
-- 此脚本删除 benefit_grants 表及其所有索引和触发器
-- 注意：执行此回滚将丢失所有权益发放记录数据

-- ============================================
-- 1. 删除触发器
-- ============================================
DROP TRIGGER IF EXISTS update_benefit_grants_updated_at ON benefit_grants;

-- ============================================
-- 2. 删除索引（DROP TABLE CASCADE 会自动删除，这里显式列出便于单独操作）
-- ============================================
DROP INDEX IF EXISTS idx_benefit_grants_user;
DROP INDEX IF EXISTS idx_benefit_grants_status;
DROP INDEX IF EXISTS idx_benefit_grants_benefit;
DROP INDEX IF EXISTS idx_benefit_grants_order;
DROP INDEX IF EXISTS idx_benefit_grants_user_status;
DROP INDEX IF EXISTS idx_benefit_grants_retry;
DROP INDEX IF EXISTS idx_benefit_grants_expires;
DROP INDEX IF EXISTS idx_benefit_grants_external_ref;
DROP INDEX IF EXISTS idx_benefit_grants_created;
DROP INDEX IF EXISTS idx_benefit_grants_source;

-- ============================================
-- 3. 删除表
-- ============================================
DROP TABLE IF EXISTS benefit_grants CASCADE;
