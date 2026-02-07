-- 批量任务失败明细表
-- 记录批量任务执行过程中失败的每一行数据

-- 添加 params 列到 batch_tasks (如果不存在)
ALTER TABLE batch_tasks
ADD COLUMN IF NOT EXISTS params JSONB;

COMMENT ON COLUMN batch_tasks.params IS '任务参数，JSON格式';

-- 创建任务失败明细表
CREATE TABLE IF NOT EXISTS batch_task_failures (
    id BIGSERIAL PRIMARY KEY,
    task_id BIGINT NOT NULL REFERENCES batch_tasks(id) ON DELETE CASCADE,
    row_number INT NOT NULL,
    user_id VARCHAR(100),
    error_code VARCHAR(50) NOT NULL,
    error_message TEXT NOT NULL,
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE batch_task_failures IS '批量任务失败明细';
COMMENT ON COLUMN batch_task_failures.row_number IS '源文件中的行号';
COMMENT ON COLUMN batch_task_failures.user_id IS '相关用户ID';
COMMENT ON COLUMN batch_task_failures.error_code IS '错误码';
COMMENT ON COLUMN batch_task_failures.error_message IS '错误描述';
COMMENT ON COLUMN batch_task_failures.raw_data IS '原始数据行';

-- 索引
CREATE INDEX IF NOT EXISTS idx_batch_task_failures_task ON batch_task_failures(task_id);
CREATE INDEX IF NOT EXISTS idx_batch_task_failures_user ON batch_task_failures(user_id);
