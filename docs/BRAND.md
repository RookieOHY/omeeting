# O-Meeting 品牌规范 v1（已定稿 2026-11-06）

## Logo：环形声波柱

8 根高低起伏的圆角音频柱围成环形（字母 O × 声纹），环心一颗圆点 = 房间里的参与者。

- **源文件**：`web/src/components/BrandLogo.vue`（参数化尺寸组件，首页顶栏 38px）
- **Favicon**：`web/public/favicon.svg`（`index.html` 已接入）
- **最小可用尺寸**：16px（几何纯矢量，无文字渲染）

## 品牌色（渐变：青 → 蓝 → 鸢尾紫）

| 色值 | 用途 |
|---|---|
| `#3AD2B5` aqua | 起始色 · 在线状态 · 「只听」标签 · 加入码 |
| `#5B8CFF` blue | 中段 · 跑马灯圆点 · 图表渐变 |
| `#8F6BFF` iris | 收尾 · ping 光效 |

CSS 里统一走 `style.css` 设计令牌：`--brand-grad` / `--aqua` / `--blue` / `--iris`，不要硬编码色值。

## 字体

- 界面：系统 sans 栈（`ui-sans-serif, -apple-system, 'PingFang SC', ...`）
- 数据（计数器/加入码）：`ui-monospace` + `tabular-nums`
