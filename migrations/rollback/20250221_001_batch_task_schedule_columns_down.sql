-- 回滚 20250221_001_batch_task_schedule_columns
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS parent_task_id;
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS reason;
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS quantity;
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS badge_id;
ALTER TABLE batch_tasks DROP COLUMN IF EXISTS name;
