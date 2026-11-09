/**
 * 会话通行证（pid）的 sessionStorage 封装。
 *
 * 通行证 = 本会话在服务器上的唯一身份，与昵称解耦：
 * - 存 sessionStorage：刷新页面保留 → 同一会话重连（服务器顶号替换旧连接）；
 * - 关标签页自动作废 → 换设备/新标签页 = 新参会人，同名也共存。
 */

const KEY_PREFIX = 'omeeting.pid.'

function key(room: string): string {
  return KEY_PREFIX + room
}

/** 取当前房间已持有的通行证（无则 undefined） */
export function loadPid(room: string): string | undefined {
  return sessionStorage.getItem(key(room)) ?? undefined
}

/** 服务器签发通行证后落盘 */
export function savePid(room: string, pid: string): void {
  sessionStorage.setItem(key(room), pid)
}

/** 会话结束（离开/ teardown）即作废，下次进房重新签发 */
export function clearPid(room: string): void {
  sessionStorage.removeItem(key(room))
}
