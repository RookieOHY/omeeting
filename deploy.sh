#!/usr/bin/env bash
# O-Meeting 一键部署脚本
# 用法：./deploy.sh user@server-ip meet.example.com 1.2.3.4
#   $1 SSH 目标（user@host）
#   $2 域名（A 记录已指向该服务器）
#   $3 服务器公网 IP
set -euo pipefail

HOST="${1:?用法: ./deploy.sh user@server-ip domain.com public-ip}"
DOMAIN="${2:?缺域名}"
PUBLIC_IP="${3:?缺公网 IP}"

echo "==> 同步源码到 ${HOST}:/opt/o-meeting"
ssh "$HOST" "sudo mkdir -p /opt/o-meeting && sudo chown \$USER /opt/o-meeting"
rsync -az --delete \
  --exclude 'web/node_modules' --exclude 'web/dist' \
  --exclude 'server/target' --exclude '.git' \
  ./ "$HOST:/opt/o-meeting/"

echo "==> 远程构建并启动（首次会拉镜像+编译，约 5-10 分钟）"
ssh "$HOST" "cd /opt/o-meeting && \
  PUBLIC_IP=${PUBLIC_IP} DOMAIN=${DOMAIN} docker compose up -d --build"

echo "==> 等待证书签发与健康检查"
sleep 8
ssh "$HOST" "curl -s -o /dev/null -w 'local https: %{http_code}\n' -k https://localhost/health || true"

echo "✅ 部署完成"
echo "   桌面浏览器：https://${DOMAIN}/room1"
echo "   手机浏览器：https://${DOMAIN}/room1 （同房间即互通）"
echo "   日志：ssh ${HOST} 'docker compose -f /opt/o-meeting/docker-compose.yml logs -f omeeting'"
