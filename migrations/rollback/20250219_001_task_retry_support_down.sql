-- 回滚 20250219_001_task_retry_support
DROP INDEX IF EXISTS idx_batch_task_failures_last_retry;
DROP INDEX IF EXISTS idx_batch_task_failures_retry;
ALTER TABLE batch_task_failures DROP COLUMN IF EXISTS retry_status;
ALTER TABLE batch_task_failures DROP COLUMN IF EXISTS last_retry_at;
ALTER TABLE batch_task_failures DROP COLUMN IF EXISTS retry_count;
