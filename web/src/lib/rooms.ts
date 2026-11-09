/** 最近房间记录，持久化在 localStorage，模拟原版 vuex-persistedstate 的行为 */

export interface RoomRecord {
  room: string
  nickname?: string
  joinedAt: number
}

const KEY = 'omeeting.rooms'

export function loadRooms(): RoomRecord[] {
  try {
    const raw = localStorage.getItem(KEY)
    if (!raw) return []
    const list = JSON.parse(raw) as RoomRecord[]
    return Array.isArray(list) ? list : []
  } catch {
    return []
  }
}

export function saveRoom(room: string, nickname?: string) {
  const list = loadRooms().filter((r) => r.room !== room)
  list.unshift({ room, nickname, joinedAt: Date.now() })
  localStorage.setItem(KEY, JSON.stringify(list.slice(0, 12)))
}

export function removeRoom(room: string) {
  const list = loadRooms().filter((r) => r.room !== room)
  localStorage.setItem(KEY, JSON.stringify(list))
}

/** 查询某房间保存过的昵称 */
export function getNickname(room: string): string | undefined {
  return loadRooms().find((r) => r.room === room)?.nickname
}
