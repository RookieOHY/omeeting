/**
 * 邀请码：房间名 → 确定性 6 位数字码。
 * 无后端阶段：同一房间在任何设备算出的码一致，用作口头核对（电话里对暗号）。
 * 未来接 Rust 后端后可升级为可反解的全局房间索引（输码进房）。
 */

/** FNV-1a 32bit，兼容中文（按 code point 迭代） */
function fnv1a(str: string): number {
  let h = 2166136261
  for (const ch of str) {
    h ^= ch.codePointAt(0)!
    h = Math.imul(h, 16777619) >>> 0
  }
  return h >>> 0
}

export function joinCode(room: string): string {
  let h = fnv1a(room.trim().toLowerCase())
  let out = ''
  for (let i = 0; i < 6; i++) {
    out += h % 10
    h = (Math.imul(h ^ 0x9e3779b9, 0x85ebca6b) >>> 0) + 0x27d4eb2f
    h = h >>> 0
  }
  return out
}

/** 口头/手输的码 → 标准比较形式：只留数字（容忍空格、横杠、全角等杂音） */
export function normalizeCode(input: string): string {
  return input.replace(/[^0-9]/g, '')
}

/** 一键分享文本：比裸链接更有社交语境 */
export function shareText(room: string, url: string, code: string): string {
  return `我在 O-Meeting 开了个语音房「#${room}」，点链接直接进：\n${url}\n（加入码 ${code}，对上了再聊）`
}
