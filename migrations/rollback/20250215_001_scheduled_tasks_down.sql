-- 回滚 20250215_001_scheduled_tasks
DROP INDEX IF EXISTS idx_batch_tasks_next_run;
DROP INDEX IF EXISTS idx_batch_tasks_scheduled;
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS next_run_at;
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS cron_expression;
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS schedule_type;
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS scheduled_at;
