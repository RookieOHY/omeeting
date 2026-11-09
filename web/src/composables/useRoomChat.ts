/**
 * 房间聊天（文字消息侧）：上屏列表、未读计数、增量合并、发送。
 *
 * 「新 vs 历史」的判据：首次 receive（chatHistoryLoaded=false）整批当作历史 ——
 * 只进聊天记录，不飞弹幕、不计未读；之后的增量才触发弹幕/未读。
 *
 * 「这是我发的吗」判据：优先按通行证 pid（同名也不误判）；服务器老消息没有
 * pid 字段时退回昵称比对。
 */
import { nextTick, ref, type Ref } from 'vue'
import { chatSend, type ChatItem } from '../lib/sfu'

export interface ChatMsg {
  id?: number
  mine: boolean
  name: string
  text: string
}

export interface RoomChatOptions {
  /** 昵称（签名回退 + 本端上屏署名） */
  nickname: Ref<string>
  /** 会话通行证 getter（判「我发的」主要依据 + 发送必带） */
  getPid: () => string | undefined
  /** 发送失败/未接入时的提示文案出口 */
  onToast: (msg: string) => void
  /** 本端发送成功后的即时弹幕（是否显示由弹幕层自己判断） */
  onOwnDanmaku: (text: string) => void
}

export function useRoomChat(opts: RoomChatOptions) {
  const messages = ref<ChatMsg[]>([])
  const chatOpen = ref(false)
  const unread = ref(0)

  let lastMsgId = 0
  let chatHistoryLoaded = false

  function append(m: ChatMsg) {
    messages.value.push(m)
    if (messages.value.length > 400) messages.value.shift() // 底部滚动 + 上限裁剪
  }

  /** 打开/关闭抽屉：打开时清未读（未读只在抽屉关着时累计） */
  function toggleChat() {
    chatOpen.value = !chatOpen.value
    if (chatOpen.value) unread.value = 0
  }

  function closeChat() {
    chatOpen.value = false
  }

  /**
   * 轮询带回的服务器消息增量（首次调用 = 历史全量）。
   * 自己的消息已在发送时上屏过，这里只补远端消息。
   */
  function receive(items: ChatItem[]) {
    const historical = !chatHistoryLoaded
    chatHistoryLoaded = true
    for (const m of items) {
      if (m.id <= lastMsgId) continue
      lastMsgId = m.id
      const mine = m.pid ? m.pid === opts.getPid() : m.user === (opts.nickname.value || '我')
      if (mine) continue
      append({ id: m.id, mine: false, name: m.user, text: m.text })
      if (!historical) {
        // 只有「新消息」才打扰：飞弹幕 + 抽屉关着时计未读
        opts.onOwnDanmaku(m.text) // 回调内部再判断弹幕开关
        if (!chatOpen.value) unread.value += 1
      }
      if (chatOpen.value) void nextTick(scrollToBottom)
    }
  }

  /**
   * 本端发送：服务器入账（按通行证署名）→ 本端立即上屏 → 即时飞幕。
   * 轮询侧 receive 会按 pid 跳过自己的消息，不会重复上屏。
   */
  async function send(text: string) {
    const t = text.trim()
    if (!t) return
    if (!opts.getPid()) {
      opts.onToast('还没接入房间，等一两秒再试')
      return
    }
    try {
      await chatSend(opts.getPid()!, t)
      append({ id: undefined, mine: true, name: opts.nickname.value || '我', text: t })
      opts.onOwnDanmaku(t)
      void nextTick(scrollToBottom)
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      opts.onToast(msg.includes('session gone') ? '连接已失效，请重新进入' : '消息没发出去，稍后再试试')
    }
  }

  /** 当前消息水位（0 = 尚未拉过），会话轮询作 chatFetch 的 since 用 */
  function getSince() {
    return lastMsgId
  }
  /** 注册滚动器：抽屉组件最清楚自己的容器，由它提供「滚到底」实现 */
  let _scroller: (() => void) | null = null
  function registerScroller(fn: () => void) {
    _scroller = fn
  }
  function scrollToBottom() {
    _scroller?.()
  }
  /** 外部（抽屉打开时）也可手动滚动 */
  function scrollToEnd() {
    scrollToBottom()
  }

  return { messages, chatOpen, unread, toggleChat, closeChat, receive, send, getSince, registerScroller, scrollToEnd }
}
