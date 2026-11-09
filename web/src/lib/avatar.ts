/**
 * 头像与状态展示的公共样式逻辑：
 * 各处（舞台瓷片、成员浮层、门禁页）共用同一套色板与算法，改一处全端生效。
 */

/** 四组低饱和渐变（白蓝系统邻近色），整场色彩克制 */
export const AVATAR_GRADS = [
  'linear-gradient(135deg, #6ea4ff 0%, #3d7bff 100%)', // 晴蓝
  'linear-gradient(135deg, #5cc6d4 0%, #2ba0b8 100%)', // 湖青
  'linear-gradient(135deg, #6cc9a4 0%, #3aa580 100%)', // 苔绿
  'linear-gradient(135deg, #9aa6e8 0%, #7583cf 100%)', // 雾紫
]

/** 头像底色：按昵称哈希从色板里稳定取色（同名人永远同色，跨端一致） */
export function nameColor(s: string): string {
  let h = 0
  for (const ch of s) h = (h * 31 + (ch.codePointAt(0) ?? 0)) >>> 0
  return `background: ${AVATAR_GRADS[h % AVATAR_GRADS.length]}`
}

/** 头像取字：中文/混排取前 2 个字符，纯英文取词首字母（John Smith → JS） */
export function avatarText(s: string): string {
  const t = (s || '我').trim()
  if (!t) return '我'
  if (/^[A-Za-z][A-Za-z\s.'-]*$/.test(t)) {
    const parts = t.split(/[\s.'-]+/).filter(Boolean)
    return parts.length >= 2 ? (parts[0][0] + parts[1][0]).toUpperCase() : t.slice(0, 2).toUpperCase()
  }
  return [...t].slice(0, 2).join('')
}

/** 说话呼吸圈：强度随真实电平（me 常亮基线，remote 仅有声时亮） */
export function ringStyle(le: number, active: boolean): string {
  if (!active) return ''
  const k = 0.18 + Math.min(1, le) * 0.82
  return [
    'box-shadow:',
    `0 0 0 3px #fff,`,
    `0 0 0 ${(3 + 5 * k).toFixed(1)}px rgba(31,107,255,${(0.22 + 0.4 * k).toFixed(2)}),`,
    `0 0 ${(10 + 22 * k).toFixed(0)}px ${(2 + 7 * k).toFixed(0)}px rgba(31,107,255,${(0.1 + 0.25 * k).toFixed(2)})`,
  ].join(' ')
}

/** 延迟点色：成员浮层里的 RTT 指示（g 绿 / o 橙 / r 红 / o 未知灰橙） */
export function pingDot(ms: number | null): string {
  if (ms == null) return 'o'
  return ms < 80 ? 'g' : ms < 200 ? 'o' : 'r'
}
