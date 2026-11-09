/** 成员浮层的行类型：浮层组件与 RoomPage（数据归一）的契约 */
export interface MemberRow {
  /** peer id；自己在 roster 里也有正式 id，'me' 仅为排序占位 */
  id: string
  name: string
  owner: boolean
  /** 是否就是我这一行（头像配色 + RTT 展示差异） */
  self: boolean
  joinedMs?: number
  /** 我才有真实 RTT；远端暂无数据通道，显示 — */
  rtt?: number | null
  speaking: boolean
  /** 心跳申报的开麦状态（副行「开麦中/静音」用，弹幕圈照旧用真电平） */
  micOn?: boolean
  /** 服务器闭麦账本：当前是否被房主闭麦（此时 micOn 显示无意义） */
  muted?: boolean
}
