/**
 * 声景引擎：用 Web Audio 程序化生成环境音（白噪音 / 雨声 / 海浪 / 篝火）。
 * 零素材依赖，浏览器本地生成；必须在用户手势后调用 play()（受自动播放策略约束）。
 */

export type SoundType = 'cafe' | 'white' | 'rain' | 'waves' | 'fire'

export const SOUNDSCAPES: Array<{ type: SoundType; label: string; icon: string; tint: string }> = [
  { type: 'cafe', label: '咖啡馆', icon: '☕', tint: '#c98a5e' },
  { type: 'white', label: '匀声', icon: '🌫️', tint: '#8f99a6' },
  { type: 'rain', label: '雨声', icon: '🌧️', tint: '#5b8cff' },
  { type: 'waves', label: '海浪', icon: '🌊', tint: '#3ad2b5' },
  { type: 'fire', label: '篝火', icon: '🔥', tint: '#ff8a3d' },
]

export class SoundScape {
  private ctx: AudioContext | null = null
  private master: GainNode | null = null
  private comp: DynamicsCompressorNode | null = null
  private live: Array<{ stop: () => void }> = []
  private buffers = new Map<SoundType, AudioBuffer>()
  current: SoundType | null = null

  /**
   * 逐声景响度校准系数：不同录音的原始响度差异很大，
   * 最终输出 = 用户音量 × 校准系数，保证任何素材在音量条满格也不会炸耳。
   */
  private static CALIB: Record<SoundType, number> = {
    cafe: 0.5,
    white: 0.38,
    rain: 0.42,
    waves: 0.45,
    fire: 0.55,
  }

  /** 真实录音 loop（Blanket 项目收录，许可见 ../assets/audio/LICENSES.md）；null = 用合成 */
  private static ASSETS: Record<SoundType, string | null> = {
    cafe: new URL('../assets/audio/coffee-shop.ogg', import.meta.url).href,
    white: null,
    rain: new URL('../assets/audio/rain.ogg', import.meta.url).href,
    waves: new URL('../assets/audio/waves.ogg', import.meta.url).href,
    fire: new URL('../assets/audio/fireplace.ogg', import.meta.url).href,
  }

  /** 惰性创建 AudioContext（需在用户手势内） */
  private ensure(): AudioContext {
    if (!this.ctx) {
      const Ctx: typeof AudioContext =
        (window as any).AudioContext || (window as any).webkitAudioContext
      this.ctx = new Ctx()
      // 防炸耳安全链：master gain → 限幅压缩器 → 扬声器
      this.master = this.ctx.createGain()
      this.master.gain.value = 0
      this.comp = this.ctx.createDynamicsCompressor()
      this.comp.threshold.value = -20
      this.comp.knee.value = 18
      this.comp.ratio.value = 8
      this.comp.attack.value = 0.01
      this.comp.release.value = 0.3
      this.master.connect(this.comp)
      this.comp.connect(this.ctx.destination)
    }
    if (this.ctx.state === 'suspended') void this.ctx.resume()
    return this.ctx
  }

  /** 目标音量 = 用户音量 × 素材校准，上限 1 */
  private targetGain(): number {
    const user = this.userVolume ?? 0.4
    const calib = SoundScape.CALIB[this.current!] ?? 0.45
    return Math.max(0, Math.min(1, user * calib))
  }

  private userVolume: number = 0.4

  setVolume(v: number) {
    this.userVolume = Math.max(0, Math.min(1, v))
    // 渐入：setTargetAtTime 从当前值平滑爬到目标，避免拖动/切声景瞬变
    if (this.master && this.ctx && this.current) {
      this.master.gain.setTargetAtTime(this.targetGain(), this.ctx.currentTime, 0.35)
    }
  }

  async play(type: SoundType) {
    this.stop()
    const ctx = this.ensure()
    this.current = type

    const url = SoundScape.ASSETS[type]
    if (url) {
      try {
        const buffer = await this.bufferFor(ctx, url, type)
        // decode 期间用户可能已切到别的声景，直接丢弃这次结果
        if (this.current !== type || !this.master) return
        const src = ctx.createBufferSource()
        src.buffer = buffer
        src.loop = true
        src.connect(this.master)
        this.fadeIn()
        src.start()
        this.track(ctx, src)
        return
      } catch {
        // 素材加载/解码失败（断网、构建异常）→ 回退到程序合成
      }
    }
    // 合成兜底（也走同样的渐入）
    switch (type) {
      case 'cafe': this.buildCafe(ctx); break
      case 'white': this.buildWhite(ctx); break
      case 'rain': this.buildRain(ctx); break
      case 'waves': this.buildWaves(ctx); break
      case 'fire': this.buildFire(ctx); break
    }
    this.fadeIn()
  }

  /** 渐入：压到近似静音再平滑爬升到目标值——换声景不炸耳 */
  private fadeIn() {
    if (!this.master || !this.ctx) return
    const t = this.ctx.currentTime
    this.master.gain.cancelScheduledValues(t)
    this.master.gain.setValueAtTime(0.0001, t)
    this.master.gain.setTargetAtTime(this.targetGain(), t, 0.45)
  }

  /** 首用 fetch + decode，之后读缓存 */
  private async bufferFor(ctx: AudioContext, url: string, type: SoundType): Promise<AudioBuffer> {
    const hit = this.buffers.get(type)
    if (hit) return hit
    const res = await fetch(url)
    if (!res.ok) throw new Error(`asset ${res.status}: ${url}`)
    const buf = await ctx.decodeAudioData(await res.arrayBuffer())
    this.buffers.set(type, buf)
    return buf
  }

  stop() {
    for (const node of this.live) node.stop()
    this.live = []
    this.current = null
  }

  /** 页面卸载时彻底释放 */
  dispose() {
    this.stop()
    void this.ctx?.close()
    this.ctx = null
    this.master = null
  }

  /* ---------- 基础材料 ---------- */

  /** 生成 2s 噪音采样缓冲，kind 控制音色（白/棕） */
  private noiseBuffer(ctx: AudioContext, kind: 'white' | 'brown', browniness = 0.98): AudioBuffer {
    const len = ctx.sampleRate * 2
    const buf = ctx.createBuffer(1, len, ctx.sampleRate)
    const data = buf.getChannelData(0)
    let last = 0
    for (let i = 0; i < len; i++) {
      const white = Math.random() * 2 - 1
      if (kind === 'white') {
        data[i] = white * 0.5
      } else {
        last = (last + browniness * white) / (1 + browniness)
        data[i] = last * 4.5
      }
    }
    return buf
  }

  private source(ctx: AudioContext, kind: 'white' | 'brown') {
    const src = ctx.createBufferSource()
    src.buffer = this.noiseBuffer(ctx, kind)
    src.loop = true
    src.start()
    return src
  }

  private track(_ctx: AudioContext, node: AudioNode) {
    const handle = {
      stop: () => {
        try { (node as any).stop?.() } catch { /* 已结束 */ }
        node.disconnect()
      }
    }
    this.live.push(handle)
    return handle
  }

  /* ---------- 各声景 ---------- */

  /** 咖啡馆：带通棕噪模拟人声嗡嗡 + 随机起伏 + 偶发杯碟叮当（合成近似，非真录音） */
  private buildCafe(ctx: AudioContext) {
    const src = this.source(ctx, 'brown')
    const hp = ctx.createBiquadFilter()
    hp.type = 'highpass'
    hp.frequency.value = 300   // 人声主要能量带下沿
    const lp = ctx.createBiquadFilter()
    lp.type = 'lowpass'
    lp.frequency.value = 1700  // 距离感：隔开清晰语音
    const murmur = ctx.createGain()
    murmur.gain.value = 0.3
    src.connect(hp).connect(lp).connect(murmur).connect(this.master!)

    // 人声忽远忽近的「呼吸起伏」
    let swellTimer!: ReturnType<typeof setTimeout>
    const swell = () => {
      if (this.current !== 'cafe' || !this.ctx) return
      murmur.gain.setTargetAtTime(0.2 + Math.random() * 0.22, ctx.currentTime, 0.7)
      swellTimer = setTimeout(swell, 900 + Math.random() * 1900)
    }
    swellTimer = setTimeout(swell, 600)
    this.live.push({ stop: () => clearTimeout(swellTimer) })
    this.track(ctx, src)

    // 杯碟叮当：稀疏的高频短音
    let clinkTimer!: ReturnType<typeof setTimeout>
    const clink = () => {
      if (this.current !== 'cafe' || !this.ctx) return
      const t = ctx.currentTime
      const osc = ctx.createOscillator()
      const g = ctx.createGain()
      osc.type = 'triangle'
      const base = 1900 + Math.random() * 1400
      osc.frequency.setValueAtTime(base, t)
      osc.frequency.exponentialRampToValueAtTime(base * 0.82, t + 0.09)
      g.gain.setValueAtTime(0.0001, t)
      g.gain.exponentialRampToValueAtTime(0.030 + Math.random() * 0.035, t + 0.005)
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.1)
      osc.connect(g).connect(this.master!)
      osc.start(t)
      osc.stop(t + 0.11)
      clinkTimer = setTimeout(clink, 2200 + Math.random() * 4200)
    }
    clinkTimer = setTimeout(clink, 1600)
    this.live.push({ stop: () => clearTimeout(clinkTimer) })
  }

  private buildWhite(ctx: AudioContext) {
    const src = this.source(ctx, 'white')
    const lp = ctx.createBiquadFilter()
    lp.type = 'lowpass'
    lp.frequency.value = 4200 // 磨掉毛刺，保留“均匀空气感”
    src.connect(lp).connect(this.master!)
    this.track(ctx, src)
  }

  private buildRain(ctx: AudioContext) {
    const ctxTime = ctx.currentTime
    const src = this.source(ctx, 'brown')
    const lp = ctx.createBiquadFilter()
    lp.type = 'lowpass'
    lp.frequency.value = 5200
    const hp = ctx.createBiquadFilter()
    hp.type = 'highpass'
    hp.frequency.value = 240
    src.connect(lp).connect(hp).connect(this.master!)

    // 雨滴：随机间隔的短促高通脉冲
    const drop = () => {
      if (this.current !== 'rain' || !this.ctx) return
      const t = ctx.currentTime
      const osc = ctx.createOscillator()
      const g = ctx.createGain()
      osc.type = 'sine'
      osc.frequency.setValueAtTime(1400 + Math.random() * 2400, t)
      g.gain.setValueAtTime(0.0001, t)
      g.gain.exponentialRampToValueAtTime(0.06 + Math.random() * 0.08, t + 0.006)
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.05)
      osc.connect(g).connect(this.master!)
      osc.start(t)
      osc.stop(t + 0.06)
      timer = setTimeout(drop, 80 + Math.random() * 190)
    }
    let timer!: ReturnType<typeof setTimeout>
    timer = setTimeout(drop, 200)
    this.live.push({ stop: () => clearTimeout(timer) })
    this.track(ctx, src)
    void ctxTime
  }

  private buildWaves(ctx: AudioContext) {
    const src = this.source(ctx, 'brown')
    const lp = ctx.createBiquadFilter()
    lp.type = 'lowpass'
    lp.frequency.value = 900
    // 缓慢的“呼吸”包络模拟浪涌
    const swell = ctx.createGain()
    swell.gain.value = 0.4
    const lfo = ctx.createOscillator()
    const depth = ctx.createGain()
    lfo.frequency.value = 0.09
    depth.gain.value = 0.55
    lfo.connect(depth).connect(swell.gain)
    lfo.start()
    src.connect(lp).connect(swell).connect(this.master!)
    this.track(ctx, src)
    this.track(ctx, lfo)
  }

  private buildFire(ctx: AudioContext) {
    const src = this.source(ctx, 'brown')
    const lp = ctx.createBiquadFilter()
    lp.type = 'lowpass'
    lp.frequency.value = 640
    src.connect(lp).connect(this.master!)

    // 柴火爆裂：随机短促的低通白噪脉冲
    const crackle = () => {
      if (this.current !== 'fire' || !this.ctx) return
      const t = ctx.currentTime
      const pulse = this.source(ctx, 'white')
      const f = ctx.createBiquadFilter()
      f.type = 'bandpass'
      f.frequency.value = 900 + Math.random() * 1600
      const g = ctx.createGain()
      g.gain.setValueAtTime(0.0001, t)
      g.gain.exponentialRampToValueAtTime(0.1 + Math.random() * 0.16, t + 0.004)
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.045)
      pulse.connect(f).connect(g).connect(this.master!)
      pulse.stop(t + 0.05)
      timer = setTimeout(crackle, 120 + Math.random() * 640)
    }
    let timer!: ReturnType<typeof setTimeout>
    timer = setTimeout(crackle, 300)
    this.live.push({ stop: () => clearTimeout(timer) })
    this.track(ctx, src)
  }
}
