-- 回滚脚本：撤销级联评估日志和分布式锁表
-- 对应 UP 迁移：20250131_001_cascade_log.sql
-- 此脚本删除级联评估日志表和分布式锁表

-- ============================================
-- 1. 删除表（按依赖关系反序，先删无依赖的表）
-- ============================================

-- 分布式锁表（无外键依赖）
DROP TABLE IF EXISTS distributed_locks CASCADE;

-- 级联评估日志（依赖 badges）
DROP TABLE IF EXISTS cascade_evaluation_logs CASCADE;
