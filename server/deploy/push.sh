#!/usr/bin/env bash
# 推送二进制到服务器并重启服务（在本地 musl 构建完成后运行）
set -euo pipefail
HOST="root@118.195.249.203"
KEY="$HOME/.ssh/omeeting_deploy"

ssh -i "$KEY" -o StrictHostKeyChecking=no "$HOST" 'mkdir -p /opt/omeeting/certs'
rsync -az -e "ssh -i $KEY -o StrictHostKeyChecking=no" \
  server/target/x86_64-unknown-linux-musl/release/omeeting-server \
  server/deploy/omeeting.service \
  "$HOST:/opt/omeeting/"

ssh -i "$KEY" -o StrictHostKeyChecking=no "$HOST" '
cp /opt/omeeting/omeeting.service /etc/systemd/system/omeeting.service
systemctl daemon-reload
systemctl enable --now omeeting
sleep 1
systemctl is-active omeeting && echo "SERVICE RUNNING"
'
