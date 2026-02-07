-- 定时任务调度支持
-- 支持立即执行、定时执行和周期执行三种模式

-- 添加调度相关列到批量任务表
ALTER TABLE batch_tasks
ADD COLUMN IF NOT EXISTS scheduled_at TIMESTAMPTZ;

ALTER TABLE batch_tasks
ADD COLUMN IF NOT EXISTS schedule_type VARCHAR(20) DEFAULT 'immediate';
-- schedule_type: immediate（立即执行）, once（定时单次）, recurring（周期执行）

ALTER TABLE batch_tasks
ADD COLUMN IF NOT EXISTS cron_expression VARCHAR(100);
-- cron 表达式用于周期执行任务

ALTER TABLE batch_tasks
ADD COLUMN IF NOT EXISTS next_run_at TIMESTAMPTZ;
-- 下次执行时间，用于周期任务调度

COMMENT ON COLUMN batch_tasks.scheduled_at IS '计划执行时间';
COMMENT ON COLUMN batch_tasks.schedule_type IS '调度类型：immediate-立即执行，once-定时单次，recurring-周期执行';
COMMENT ON COLUMN batch_tasks.cron_expression IS 'Cron 表达式（仅周期任务使用）';
COMMENT ON COLUMN batch_tasks.next_run_at IS '下次执行时间（周期任务）';

-- 索引：用于轮询待执行的定时任务
CREATE INDEX IF NOT EXISTS idx_batch_tasks_scheduled
ON batch_tasks(scheduled_at)
WHERE scheduled_at IS NOT NULL AND status = 'pending';

-- 索引：用于轮询周期任务的下次执行
CREATE INDEX IF NOT EXISTS idx_batch_tasks_next_run
ON batch_tasks(next_run_at)
WHERE next_run_at IS NOT NULL AND schedule_type = 'recurring';
