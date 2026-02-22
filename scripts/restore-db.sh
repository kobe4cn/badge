#!/usr/bin/env bash
# PostgreSQL 数据库恢复脚本
#
# 用法：
#   ./scripts/restore-db.sh <backup_file>
#   ./scripts/restore-db.sh backups/badge_backup_20260222_020000.dump
#
# 恢复前会自动创建当前数据库的快照备份，以便回滚。
#
# 环境变量：
#   DB_HOST          数据库主机（默认 localhost）
#   DB_PORT          数据库端口（默认 5432）
#   DB_NAME          数据库名称（默认 badge_db）
#   DB_USER          数据库用户（默认 badge）
#   PGPASSWORD       数据库密码
#   BACKUP_DIR       备份目录（默认 ./backups）
#   SKIP_PRE_BACKUP  设为 "true" 跳过恢复前备份

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "用法: $0 <backup_file>"
  echo "示例: $0 backups/badge_backup_20260222_020000.dump"
  exit 1
fi

BACKUP_FILE="$1"

if [ ! -f "${BACKUP_FILE}" ]; then
  echo "ERROR: 备份文件不存在: ${BACKUP_FILE}"
  exit 1
fi

DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-badge_db}"
DB_USER="${DB_USER:-badge}"
BACKUP_DIR="${BACKUP_DIR:-./backups}"
SKIP_PRE_BACKUP="${SKIP_PRE_BACKUP:-false}"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"; }

# 验证备份文件
log "验证备份文件: ${BACKUP_FILE}"
if ! pg_restore --list "${BACKUP_FILE}" > /dev/null 2>&1; then
  log "ERROR: 备份文件无效或已损坏"
  exit 1
fi

TABLE_COUNT=$(pg_restore --list "${BACKUP_FILE}" 2>/dev/null | grep -c "TABLE " || true)
log "备份文件有效，包含 ${TABLE_COUNT} 个表"

# 恢复前创建当前数据库快照（防止误操作导致数据丢失）
if [ "${SKIP_PRE_BACKUP}" != "true" ]; then
  mkdir -p "${BACKUP_DIR}"
  PRE_RESTORE_FILE="${BACKUP_DIR}/badge_pre_restore_$(date +%Y%m%d_%H%M%S).dump"
  log "创建恢复前快照: ${PRE_RESTORE_FILE}"
  pg_dump \
    -h "${DB_HOST}" \
    -p "${DB_PORT}" \
    -U "${DB_USER}" \
    -d "${DB_NAME}" \
    --no-owner \
    --no-privileges \
    --format=custom \
    --compress=6 \
    -f "${PRE_RESTORE_FILE}" || {
      log "WARN: 恢复前快照失败（数据库可能为空），继续恢复"
    }
fi

log "开始恢复 ${BACKUP_FILE} -> ${DB_NAME}@${DB_HOST}:${DB_PORT}"

# --clean 先删除现有对象再恢复，--if-exists 避免对象不存在时报错
pg_restore \
  -h "${DB_HOST}" \
  -p "${DB_PORT}" \
  -U "${DB_USER}" \
  -d "${DB_NAME}" \
  --no-owner \
  --no-privileges \
  --clean \
  --if-exists \
  "${BACKUP_FILE}"

log "恢复完成"

# 验证恢复结果
RESTORED_TABLES=$(psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" -tAc \
  "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public'" 2>/dev/null || echo "unknown")
log "恢复后数据库包含 ${RESTORED_TABLES} 个表"
