/**
 * 音频播放解锁：iOS/安卓浏览器要求「用户手势内」放一段声音，
 * 之后页面里的 audio（远端语音）才允许自动播放。加入房间必然经过一次点击，
 * 就在手势里把通道打开。
 */

/** 静音 wav 空放一次：激活浏览器的音频输出通道 */
export function unlockPlayback(): void {
  const a = document.createElement('audio')
  a.setAttribute('playsinline', 'true')
  a.src = 'data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQAAAAA='
  void a.play().catch(() => {})
}

/**
 * 逐个尝试播放页面里的远端 audio。
 * @returns true = 有被系统拦截的（需要用户再点一次解锁）
 */
export async function ensureRemoteAudio(): Promise<boolean> {
  const els = [...document.querySelectorAll<HTMLAudioElement>('audio[autoplay]')]
  let blocked = false
  for (const el of els) {
    if (el.paused) {
      try {
        await el.play()
      } catch {
        blocked = true
      }
    }
  }
  return blocked
}
