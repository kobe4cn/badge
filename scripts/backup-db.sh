#!/usr/bin/env bash
# PostgreSQL 数据库备份脚本
#
# 用法：
#   ./scripts/backup-db.sh
#
# 环境变量：
#   DB_HOST          数据库主机（默认 localhost）
#   DB_PORT          数据库端口（默认 5432）
#   DB_NAME          数据库名称（默认 badge）
#   DB_USER          数据库用户（默认 badge）
#   BACKUP_DIR       备份目录（默认 ./backups）
#   RETENTION_DAYS   保留天数（默认 7）

set -euo pipefail

DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-badge_db}"
DB_USER="${DB_USER:-badge}"
BACKUP_DIR="${BACKUP_DIR:-./backups}"
RETENTION_DAYS="${RETENTION_DAYS:-7}"

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/badge_backup_${TIMESTAMP}.sql.gz"

mkdir -p "${BACKUP_DIR}"

echo "[$(date)] 开始备份 ${DB_NAME}@${DB_HOST}:${DB_PORT} ..."

pg_dump \
  -h "${DB_HOST}" \
  -p "${DB_PORT}" \
  -U "${DB_USER}" \
  -d "${DB_NAME}" \
  --no-owner \
  --no-privileges \
  --format=plain \
  | gzip > "${BACKUP_FILE}"

FILESIZE=$(du -h "${BACKUP_FILE}" | cut -f1)
echo "[$(date)] 备份完成：${BACKUP_FILE} (${FILESIZE})"

# 清理超过保留天数的旧备份
DELETED=$(find "${BACKUP_DIR}" -name "badge_backup_*.sql.gz" -mtime +"${RETENTION_DAYS}" -print -delete | wc -l | tr -d ' ')
if [ "${DELETED}" -gt 0 ]; then
  echo "[$(date)] 已清理 ${DELETED} 个超过 ${RETENTION_DAYS} 天的旧备份"
fi

echo "[$(date)] 备份完毕"
