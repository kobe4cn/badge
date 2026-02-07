-- 批量任务失败重试支持
-- 扩展 batch_task_failures 表，增加重试状态跟踪

-- 添加重试次数
ALTER TABLE batch_task_failures
ADD COLUMN IF NOT EXISTS retry_count INT NOT NULL DEFAULT 0;

-- 添加上次重试时间
ALTER TABLE batch_task_failures
ADD COLUMN IF NOT EXISTS last_retry_at TIMESTAMPTZ;

-- 添加重试状态: PENDING-待重试, RETRYING-重试中, SUCCESS-重试成功, EXHAUSTED-重试次数已耗尽
ALTER TABLE batch_task_failures
ADD COLUMN IF NOT EXISTS retry_status VARCHAR(20) NOT NULL DEFAULT 'PENDING';

COMMENT ON COLUMN batch_task_failures.retry_count IS '已重试次数';
COMMENT ON COLUMN batch_task_failures.last_retry_at IS '上次重试时间';
COMMENT ON COLUMN batch_task_failures.retry_status IS '重试状态：PENDING-待重试，RETRYING-重试中，SUCCESS-重试成功，EXHAUSTED-已耗尽';

-- 索引：用于查询待重试的失败记录
CREATE INDEX IF NOT EXISTS idx_batch_task_failures_retry
ON batch_task_failures(task_id, retry_status)
WHERE retry_status IN ('PENDING', 'RETRYING');

-- 索引：用于按最后重试时间排序
CREATE INDEX IF NOT EXISTS idx_batch_task_failures_last_retry
ON batch_task_failures(last_retry_at)
WHERE retry_status = 'PENDING';
