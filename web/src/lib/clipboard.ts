/**
 * 复制到剪贴板：统一走 navigator.clipboard，权限被拒时静默失败。
 * 调用方用返回的 boolean 驱动「已复制 ✓」的按钮反馈。
 */

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    return false
  }
}

/** 短暂点亮一个布尔标志位（按钮反馈用），ms 后自动熄灭 */
export function flash(flag: { value: boolean }, ms = 1600): void {
  flag.value = true
  setTimeout(() => (flag.value = false), ms)
}
