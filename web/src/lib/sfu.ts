/**
 * SFU 信令客户端：kraken 兼容的 5 方法 JSON-RPC。
 *
 * turn      [userId]                              → RTCIceServer[]
 * publish   [room, user, sdpOffer(JSON 字符串)]    → { sdp: SdpAnswer, track: string }
 * subscribe [room, user, ctrlTrackId]              → SdpOffer | null（有新轨时下发 offer）
 * answer    [room, user, ctrlTrackId, sdpAnswer]   → null
 * trickle   [room, user, ctrlTrackId, candidate]   → null
 *
 * 开麦房专用：纯收听房不 publish（str0m 侧也就没有该 peer）。
 */

/**
 * SFU 地址：默认走同源 /rpc（Vite 代理转发，免去自签证书二次信任）。
 * 直连模式可设 VITE_SFU_BASE（如 https://192.168.31.52:7070）。
 */
const env = (import.meta as unknown as { env?: Record<string, string> }).env
export const SFU_BASE = env?.VITE_SFU_BASE_MODE === 'direct' ? (env.VITE_SFU_BASE ?? '') : ''

type Envelope = { id: string; data?: unknown; error?: { code: number; description: string } }

let seq = 0

/** dev 探针：startMic 步进跟踪 */
const micTrace: string[] = []
if (!import.meta.env.PROD) {
  ;(window as unknown as Record<string, unknown>).__micTrace = micTrace
  ;(window as unknown as Record<string, unknown>).__sfuVersion = 'sfu-v3-race-trace'
}
function trace(s: string) {
  micTrace.push(`${Date.now() % 100000} ${s}`)
}

async function rpc<T>(method: string, params: unknown[]): Promise<T> {
  const res = await fetch(`${SFU_BASE}/rpc`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id: `${Date.now()}-${seq++}`, method, params }),
  })
  if (!res.ok) throw new Error(`rpc http ${res.status}`)
  const body = (await res.json()) as Envelope
  if (body.error) throw new Error(`rpc ${method}: ${body.error.description}`)
  return body.data as T
}

/** 房间花名册：远端席位卡用数字 peer id 对应昵称 */
export async function fetchRoster(room: string): Promise<Array<{ id: string; name: string; owner?: boolean; joinedMs?: number }>> {
  return rpc('roster', [room])
}

/** 聊天/弹幕消息入服务器房间缓冲（按通行证署名），返回消息 id */
export async function chatSend(pid: string, text: string): Promise<number> {
  const r = await rpc<{ id: number }>('chatSend', [pid, text])
  return r.id
}

export interface ChatItem { id: number; user: string; text: string; ts: number; pid?: string }

/** 增量拉取房间消息（id > since） */
export async function chatFetch(room: string, since: number): Promise<ChatItem[]> {
  return rpc('chatFetch', [room, String(since)])
}

/** 无麦克风/被拒时的兜底音源：440Hz（level=0 则为纯静音轨，驱动通道但不发声） */
export async function syntheticTrack(level = 0.04): Promise<MediaStreamTrack> {
  const ctx = new AudioContext()
  const osc = ctx.createOscillator()
  const gain = ctx.createGain()
  const dst = ctx.createMediaStreamDestination()
  gain.gain.value = level
  osc.frequency.value = 440
  osc.connect(gain).connect(dst)
  osc.start()
  return dst.stream.getAudioTracks()[0]
}

import { createLevelMeter, type LevelMeter } from './level'

export type TrackSource = 'silent' | 'tone' | 'mic'

export interface MicHandle {
  stop(): void
  /** 动态替换发布中的音轨（无需重新协商）：silent=静音保活 tone=测试音 mic=真实麦克风 */
  setSource(src: TrackSource): Promise<void>
  /** 当前发布的是否合成轨（并带可听电平） */
  synthetic: boolean
  /** 当前发布是否为静音保活轨（= 对方听不到你，但你仍在听房间） */
  muted: boolean
  /** 本端发布轨的实时音量 0..1（扬声器条用；静音轨≈0） */
  level: number
  /** 自己到 SFU 的往返延迟 ms（失败/未知为 null） */
  rtt(): Promise<number | null>
}

/**
 * 开麦：采集麦克风 → publish → 订阅循环接收他人音频。
 * 返回句柄用于停止（关轨、关 pc、停轮询）。
 */
export async function startMic(opts: {
  room: string
  user: string
  /** 通行证：sessionStorage 里上一次会话的 pid（刷新重连用），无则服务器签发 */
  token?: string
  /** 服务器签发通行证后回调（存 sessionStorage） */
  onPid?: (pid: string) => void
  /** 初始发布轨：'silent' = 静音保活（加入即听模式）；'tone' = 测试音；'real' = 真实麦克风 */
  initial?: TrackSource
  /** 兼容旧参数（'tone'|'real' 二选一） */
  source?: 'auto' | 'synthetic'
  onRemoteTrack?: (stream: MediaStream, fromUser: string) => void
  onRemoteGone?: (fromUser: string) => void
  onError?: (err: unknown) => void
  /** getUserMedia 失败时的兜底回调 */
  onFallback?: (reason: string) => void
}): Promise<MicHandle> {
  const { room, user, onRemoteTrack, onError } = opts
  trace(`start ${room} ${user}`)

  // ICE：客户端内置 STUN（局域网 host candidate 即可互通）。
  // rpc('turn') 保留在服务端协议里，等未来下发 TURN 临时凭证时再启用。
  const DEFAULT_ICE: RTCIceServer[] = [
    { urls: 'stun:stun.l.google.com:19302' },
    { urls: 'stun:stun1.l.google.com:19302' },
  ]
  const pc = new RTCPeerConnection({ iceServers: DEFAULT_ICE })
  trace('pc created')
  pc.onconnectionstatechange = () => trace(`pc state: ${pc.connectionState}`)
  pc.oniceconnectionstatechange = () => trace(`ice: ${pc.iceConnectionState}`)

  const ctrl = `${user}:${Math.random().toString(36).slice(2, 8)}`
  let stopped = false
  let curSource: TrackSource = 'silent'
  let pollTimer: number | undefined

  // 通行证：publish 前可能还没有（首进房），candidate 先排队，拿到 pid 再补交
  let pid: string | undefined = opts.token
  const pendingCands: RTCIceCandidate[] = []

  const remoteByUser = new Map<string, MediaStream>()
  let meter: LevelMeter | null = null

  function attachMeter(track: MediaStreamTrack | undefined) {
    meter?.dispose()
    meter = track ? createLevelMeter(track) : null
  }

  pc.onicecandidate = ({ candidate }) => {
    if (!candidate) return
    if (!pid) {
      pendingCands.push(candidate)
      return
    }
    rpc('trickle', [pid, ctrl, JSON.stringify(candidate)]).catch(() => {})
  }

  pc.ontrack = ({ streams }) => {
    const stream = streams[0]
    trace(`ontrack: ${stream?.id}`)
    // str0m 的 stream_id = add_media 时的 origin id（peer id）
    // v1：以 stream.id 作为远端标识回调
    onRemoteTrack?.(stream, stream.id)
    if (stream.id) remoteByUser.set(stream.id, stream)
  }

  try {
    let local: MediaStream
    let cur: TrackSource

    const initial: TrackSource = opts.initial ?? (opts.source === 'auto' ? 'mic' : 'tone')
    trace(`initial source: ${initial}`)

    if (initial === 'silent') {
      // 加入即听模式：发布一路零电平静音轨，只下行不干扰任何人
      cur = 'silent'
      local = new MediaStream([await syntheticTrack(0)])
    } else if (initial === 'mic') {
      // 真实麦克风：4s 无响应则回退测试音源
      trace('gum requested')
      const gUM = navigator.mediaDevices
        .getUserMedia({ audio: true, video: false })
        .then((s) => ({ s, synthetic: false }))
        .catch(() => null)
      const got = await Promise.race([
        gUM,
        new Promise<null>((r) => setTimeout(() => r(null), 4000)),
      ])
      trace(`gum resolved: ${got ? 'real' : 'fallback'}`)
      if (got) {
        cur = 'mic'
        local = got.s
      } else {
        opts.onFallback?.('麦克风无响应（权限待定或无设备）')
        cur = 'tone'
        local = new MediaStream([await syntheticTrack()])
      }
    } else {
      // 确定性路径：不碰 getUserMedia，零弹窗零挂起
      trace('tone source requested')
      cur = 'tone'
      local = new MediaStream([await syntheticTrack()])
    }
    curSource = cur
    local.getTracks().forEach((t) => pc.addTrack(t, local))
    attachMeter(local.getTracks()[0])

    const offer = await pc.createOffer()
    await pc.setLocalDescription(offer)
    // 服务端返回 { sdp: {type:'answer', sdp:'v=0...'}, track: userId, pid: 通行证 }
    const ans = await rpc<{ sdp: RTCSessionDescriptionInit; track: string; pid: string }>('publish', [
      room,
      user,
      JSON.stringify(pc.localDescription),
      pid ?? '',
    ])
    pid = ans.pid
    opts.onPid?.(pid)
    trace(`publish ok pid=${pid}`)
    await pc.setRemoteDescription(new RTCSessionDescription(ans.sdp))
    trace('remote description ok')
    // 补交 publish 期间产生的 candidate
    for (const c of pendingCands.splice(0)) {
      rpc('trickle', [pid, ctrl, JSON.stringify(c)]).catch(() => {})
    }
  } catch (err) {
    onError?.(err)
    throw err
  }

  // 动态换轨（腾讯会议模式）：不重新协商，仅替换发送端音轨
  async function setSource(src: TrackSource): Promise<void> {
    if (stopped) return
    if (src === curSource) return
    const sender = pc.getSenders().find((s) => s.track?.kind === 'audio')
    if (!sender || !sender.track) throw new Error('no audio sender')
    let next: MediaStreamTrack
    if (src === 'mic') {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false })
      next = stream.getAudioTracks()[0]
      if (!next) throw new Error('no mic track')
    } else {
      next = await syntheticTrack(src === 'silent' ? 0 : 0.04)
    }
    const prev = sender.track
    await sender.replaceTrack(next)
    prev.stop() // 旧轨无论真假都由我们独占发布，直接释放
    attachMeter(next)
    curSource = src
    trace(`source -> ${src}`)
  }

  // 订阅循环：3s 一次，有 offer 就协商。
  // 订阅偶发抖动（网络/服务端瞬时错误）不算致命，静默重试；
  // 连续 ~18s 都失败才判定连线死亡。但「会话已被顶/服务器重启」立刻致命。
  let subFail = 0
  async function pollOnce() {
    if (stopped) return
    try {
      const offer = await rpc<RTCSessionDescriptionInit | null>('subscribe', [pid, ctrl])
      if (offer && !stopped) {
        await pc.setRemoteDescription(new RTCSessionDescription(offer))
        const answer = await pc.createAnswer()
        await pc.setLocalDescription(answer)
        await rpc('answer', [pid, ctrl, JSON.stringify(pc.localDescription)])
      }
      subFail = 0
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      if (msg.includes('session gone') || msg.includes('peer not found')) {
        // 通行证失效：服务器已不认识这个会话，静默重试无意义，立刻通知上层重进
        trace(`session gone: ${msg}`)
        onError?.(err)
        return
      }
      subFail += 1
      if (subFail >= 6) {
        subFail = 0
        onError?.(err)
      }
    }
    if (!stopped) pollTimer = window.setTimeout(pollOnce, 3000)
  }
  pollTimer = window.setTimeout(pollOnce, 300)

  return {
    stop() {
      stopped = true
      if (pollTimer) clearTimeout(pollTimer)
      meter?.dispose()
      meter = null
      pc.getSenders().forEach((s) => s.track?.stop())
      pc.close()
    },
    setSource,
    get synthetic() {
      return curSource === 'tone'
    },
    get muted() {
      return curSource === 'silent'
    },
    get level() {
      return meter?.level() ?? 0
    },
    /** 自己到 SFU 的往返延迟（成员面板用） */
    async rtt(): Promise<number | null> {
      try {
        const stats = await pc.getStats()
        let ms: number | null = null
        stats.forEach((r) => {
          if (r.type === 'candidate-pair' && r.state === 'succeeded' && r.currentRoundTripTime != null) {
            ms = Math.round(r.currentRoundTripTime * 1000)
          }
        })
        return ms
      } catch {
        return null
      }
    },
  }
}
