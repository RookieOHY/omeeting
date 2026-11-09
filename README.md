# O-Meeting

开箱即用的免费语音会议室：**一条链接拉所有人进来**，不用注册、不用装 App。

自研 Rust WebRTC SFU（基于 [str0m](https://github.com/lookback/str0m)）+ Vue 3 前端，单个二进制同时承载网页、信令和音频转发。音频实时转发、不混流、**不落盘**。

```
手机/电脑浏览器 ──HTTPS──▶ Caddy(443, 自动证书) ──▶ omeeting(Rust: 7070 信令+内嵌前端)
        └──────────────── UDP 7071 ──────────────▶ omeeting（WebRTC 媒体直连）
```

## 功能

- **一条链接即房间**：`https://你的域名/房间名` 就是全部，点开就进
- **按住说话**：长按按钮临时开麦、单击持续开麦、空格键 PTT（电脑端）
- **聊天 + 弹幕**：同一个消息通道——聊天抽屉看谁发了什么（服务器暂存 200 条/房），开弹幕时消息飞过屏幕；未读红点角标
- **房主管理**：第一个进房的人是房主，可「结束房间」（成员浮层看加入时间、房主标记）
- **成员浮层**：在线人数、花名册、实时说话状态
- **移动端适配**：单条图标化工具栏、触控目标 ≥ 40px、贴 iOS 安全区
- **邀请码**：房间名确定性算出 6 位数字码，口头核对用
- **环境声景**：咖啡馆 / 雨 / 浪 / 火 / 篝火真实录音循环（可选氛围）
- **加入即听**：进房默认静音收听，点「说话」才上麦

## 快速开始

### 本地开发

```bash
# 1) 前端 dev server（/rpc 代理到本机 SFU）
cd web && npm install && npm run dev

# 2) Rust SFU（自签证书跑 HTTPS:7070，媒体 UDP:7071）
cd server && cargo run
```

打开 `http://localhost:5173/房间名`。手机测试需要域名+受信证书，见 `docs/DEPLOY.md`。

### 服务器一键部署

```bash
./deploy.sh user@server-ip meet.example.com 1.2.3.4
```

要求：1 核 1G 起的 VPS、Docker、防火墙放行 **TCP 80 / TCP 443 / UDP 7071**、一个解析到服务器 IP 的域名（手机麦克风必须受信证书）。细节与常见问题见 **[docs/DEPLOY.md](docs/DEPLOY.md)**。

### 日常发版（已有部署）

```bash
cd web && npm run build
scp -r dist/. user@server:/opt/omeeting/web/dist/
scp server/src/*.rs user@server:/opt/omeeting/server/src/
ssh user@server 'cd /opt/omeeting/server && cargo build --release && systemctl restart omeeting'
```

> ⚠️ 前端 dist 是**编译期**嵌入二进制的（rust-embed）：只改前端也必须 `cargo build --release` 后重启，否则线上还是旧页面。

## 仓库结构

```
server/        Rust SFU（str0m 引擎 + JSON-RPC 信令 + 静态文件内嵌）
  src/engine.rs   房间/peer 管理、音频转发、房主登记、聊天缓冲、顶号去重
  src/rpc.rs      JSON-RPC：publish/subscribe/answer/trickle/roster/chatSend/chatFetch
  deploy/         systemd 服务文件
web/           Vue 3 + Vite 前端
  src/pages/RoomPage.vue   房间页（工具栏/瓷片/聊天/弹幕/成员）
  src/lib/sfu.ts           WebRTC 发布/订阅循环、聊天 RPC 客户端
design/        定稿设计草图（静态 HTML）
docs/          部署指引、品牌、路线图
```

## 技术要点

- **单端口媒体**：str0m 单 UDP 端口复用所有房间的 ICE 连接，防火墙只需开一个 UDP 口
- **顶号去重**：同一房间同一昵称重新连接自动替换旧 peer，重复点「说话」人数不虚增
- **订阅容错**：订阅轮询抖动静默重试，连续 ~18s 失败才判定断线，避免一次网络抖动拆掉整条语音
- **聊天即弹幕**：一个消息通道两种呈现，服务器环形缓冲 200 条/房，房间清空自动删除
- **确定性邀请码**：FNV-1a(房间名) → 6 位数字，任何设备算出的码一致，无需存库

## Roadmap

见 [docs/ROADMAP.md](docs/ROADMAP.md)。近期：房主闭麦指令、麦状态心跳申报、结束房间真解散、系统消息、TURN 中继（严格 NAT）。

## License

仅供学习与自用部署。第三方音频素材许可见 [web/src/assets/audio/LICENSES.md](web/src/assets/audio/LICENSES.md)。
