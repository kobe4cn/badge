-- 批量任务表补充字段
-- scheduled_task_worker 创建子任务时需要 name、badge_id、quantity、reason、parent_task_id 列，
-- 这些列在初始 schema 中缺失，导致子任务 INSERT 失败

ALTER TABLE batch_tasks ADD COLUMN IF NOT EXISTS name VARCHAR(200);
ALTER TABLE batch_tasks ADD COLUMN IF NOT EXISTS badge_id BIGINT;
ALTER TABLE batch_tasks ADD COLUMN IF NOT EXISTS quantity INT DEFAULT 1;
ALTER TABLE batch_tasks ADD COLUMN IF NOT EXISTS reason TEXT;
ALTER TABLE batch_tasks ADD COLUMN IF NOT EXISTS parent_task_id BIGINT REFERENCES batch_tasks(id);
