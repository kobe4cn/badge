-- 回滚脚本：撤销批量任务失败明细表及 batch_tasks 表扩展
-- 对应 UP 迁移：20250210_001_batch_task_failures.sql
-- 此脚本删除失败明细表，并移除 batch_tasks 表上新增的列

-- ============================================
-- 1. 删除批量任务失败明细表
-- ============================================
DROP TABLE IF EXISTS batch_task_failures CASCADE;

-- ============================================
-- 2. 移除 batch_tasks 表上新增的列
-- ============================================
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS params;
