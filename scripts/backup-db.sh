#!/usr/bin/env bash
# PostgreSQL 数据库备份脚本
#
# 用法：
#   ./scripts/backup-db.sh
#
# 环境变量：
#   DB_HOST          数据库主机（默认 localhost）
#   DB_PORT          数据库端口（默认 5432）
#   DB_NAME          数据库名称（默认 badge_db）
#   DB_USER          数据库用户（默认 badge）
#   PGPASSWORD       数据库密码（pg_dump 通过此变量读取）
#   BACKUP_DIR       备份目录（默认 ./backups）
#   RETENTION_DAYS   保留天数（默认 7）
#   S3_BUCKET        可选，上传到 S3/OSS 的桶名

set -euo pipefail

DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-badge_db}"
DB_USER="${DB_USER:-badge}"
BACKUP_DIR="${BACKUP_DIR:-./backups}"
RETENTION_DAYS="${RETENTION_DAYS:-7}"
S3_BUCKET="${S3_BUCKET:-}"

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
# custom 格式支持 pg_restore --list 验证和选择性恢复
BACKUP_FILE="${BACKUP_DIR}/badge_backup_${TIMESTAMP}.dump"

mkdir -p "${BACKUP_DIR}"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"; }

log "开始备份 ${DB_NAME}@${DB_HOST}:${DB_PORT} ..."

pg_dump \
  -h "${DB_HOST}" \
  -p "${DB_PORT}" \
  -U "${DB_USER}" \
  -d "${DB_NAME}" \
  --no-owner \
  --no-privileges \
  --format=custom \
  --compress=6 \
  -f "${BACKUP_FILE}"

FILESIZE=$(du -h "${BACKUP_FILE}" | cut -f1)
log "备份完成：${BACKUP_FILE} (${FILESIZE})"

# 验证备份文件完整性：pg_restore --list 能读取 TOC 即说明文件未损坏
log "验证备份文件完整性..."
if pg_restore --list "${BACKUP_FILE}" > /dev/null 2>&1; then
  TABLE_COUNT=$(pg_restore --list "${BACKUP_FILE}" 2>/dev/null | grep -c "TABLE " || true)
  log "验证通过，备份包含 ${TABLE_COUNT} 个表"
else
  log "ERROR: 备份文件验证失败！"
  exit 1
fi

# 上传到对象存储（如果配置了 S3_BUCKET）
if [ -n "${S3_BUCKET}" ]; then
  S3_KEY="badge-backups/$(date +%Y/%m)/badge_backup_${TIMESTAMP}.dump"
  log "上传备份到 s3://${S3_BUCKET}/${S3_KEY} ..."
  if command -v aws &>/dev/null; then
    aws s3 cp "${BACKUP_FILE}" "s3://${S3_BUCKET}/${S3_KEY}"
    log "S3 上传完成"
  else
    log "WARN: aws CLI 未安装，跳过 S3 上传"
  fi
fi

# 清理超过保留天数的旧备份
DELETED=$(find "${BACKUP_DIR}" -name "badge_backup_*.dump" -mtime +"${RETENTION_DAYS}" -print -delete 2>/dev/null | wc -l | tr -d ' ')
if [ "${DELETED}" -gt 0 ]; then
  log "已清理 ${DELETED} 个超过 ${RETENTION_DAYS} 天的旧备份"
fi

log "备份完毕"
