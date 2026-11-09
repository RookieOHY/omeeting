/**
 * 弹幕层状态：飞行条目池 + 输入条开关。
 *
 * 「弹幕显示」默认常开（谁发的都飞过屏幕）；工具栏上的弹幕按钮只负责
 * 打开/收起输入条，不控制显示层 —— 这是产品定案，别把两个开关合并。
 */
import { ref } from 'vue'

export interface DanmakuItem {
  id: number
  text: string
  /** 顶部偏移百分比 12–68，伪随机（同一批不撞行） */
  top: number
}

/** 同屏最多 12 条，再多就丢弃最旧的（防轰炸） */
const MAX_FLYING = 12

export function useDanmaku(onSend: (text: string) => void) {
  const danmakuOn = ref(true) // 显示层开关（默认常开，暂无隐藏入口）
  const dmOpen = ref(false) // 输入条
  const dmText = ref('')
  const items = ref<DanmakuItem[]>([])

  let seq = 0

  /** 一条弹幕起飞（发送/远端到达都会调这里） */
  function push(text: string) {
    if (!danmakuOn.value) return
    const id = ++seq
    const top = 12 + ((id * 37) % 56) // 12%–68% 伪随机高度
    items.value.push({ id, text, top })
    if (items.value.length > MAX_FLYING) items.value.shift()
  }

  /** 飞完出屏（animationend）后回收 */
  function remove(id: number) {
    items.value = items.value.filter((d) => d.id !== id)
  }

  /** 输入条回车：发射并清空 */
  function send() {
    const t = dmText.value.trim()
    if (!t) return
    dmText.value = ''
    onSend(t) // 发送链路由房间里间层处理（入账 + 本端上屏 + 立即 push）
  }

  return { danmakuOn, dmOpen, dmText, items, push, remove, send }
}
