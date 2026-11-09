/* 音量表：WebAudio AnalyserNode RMS → 0..1
 * 本端（麦克风）与远端（SFU 下行）通用；只分析不外放（不给 destination 连接），
 * 所以不会产生回声或双放。 */

export interface LevelMeter {
  /** 0..1，已做平滑；不说话 ≈ 0 */
  level(): number
  dispose(): void
}

export function createLevelMeter(track: MediaStreamTrack): LevelMeter {
  const ctx = new AudioContext()
  const src = ctx.createMediaStreamSource(new MediaStream([track]))
  const analyser = ctx.createAnalyser()
  analyser.fftSize = 512
  src.connect(analyser)
  const buf = new Uint8Array(analyser.fftSize)
  let smooth = 0

  return {
    level() {
      if (ctx.state === 'suspended') void ctx.resume()
      analyser.getByteTimeDomainData(buf)
      let sum = 0
      for (let i = 0; i < buf.length; i++) {
        const v = (buf[i] - 128) / 128
        sum += v * v
      }
      const rms = Math.sqrt(sum / buf.length)
      // 电平增益：一般语音 RMS 约 0.05~0.3 ×4 后满条可及
      const norm = Math.min(1, rms * 4)
      smooth += (norm - smooth) * 0.35
      return smooth
    },
    dispose() {
      void ctx.close().catch(() => {})
    },
  }
}
