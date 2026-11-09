# 部署指引

## 架构

```
手机/电脑浏览器 ──HTTPS(443, Caddy自动LE证书)──▶ Caddy ──▶ omeeting(Rust SFU:7070, 内嵌前端+信令)
                    │
                    └──UDP 7071──▶ omeeting（WebRTC 媒体直连）
```

- **一个 Rust 二进制**内嵌前端 dist + JSON-RPC 信令 + str0m SFU 媒体转发
- Caddy 负责域名证书（Let's Encrypt 自动签发续期），SFU 本身跑 HTTP
- 媒体 UDP 7071 直连服务器（str0m 单端口复用所有房间的 ICE）

## 服务器要求

- 任意 1 核 1G 起的 VPS（音频转发 CPU 占用极低）
- 已安装 Docker + docker compose 插件
- 防火墙/安全组放行：**TCP 80、TCP 443、UDP 7071**
- 一个域名，A 记录指向服务器公网 IP（手机麦克风必须要受信任证书，域名是前提）

## 部署步骤

```bash
./deploy.sh user@server-ip meet.example.com 1.2.3.4
```

然后所有设备打开 `https://meet.example.com/房间名`。

## 环境变量

| 变量 | 说明 |
|---|---|
| PUBLIC_IP | 服务器公网 IP（ICE 候选宣告，必填）|
| DOMAIN | 域名（Caddy 签证书用，必填）|
| HTTP_PORT | 默认 7070（容器内）|
| MEDIA_PORT | 默认 7071（容器内，映射宿主机 7071/udp）|

## 无域名时的过渡方案

- Android Chrome：`chrome://flags/#unsafely-treat-insecure-origin-as-secure` 加入 `http://IP:7070`
- iOS Safari：没有 flag，**必须**域名+受信证书
- 结论：想认真测，域名是绕不开的最短路径（免费方案：DuckDNS 子域名）

## 常见问题

- **ICE 连不上**：检查 UDP 7071 是否放行；`PUBLIC_IP` 是否为真实公网 IP（NAT 后必须手动指定）
- **对方听不到我**：检查双方都点了「开麦」；浏览器地址栏必须是 `https://`
- **严格 NAT 环境**（公司网/4G）：需要 TURN 服务器（coturn），后续可加
