/**
 * 房间语音会话：SFU 发布/订阅、通行证、麦克风开关（PTT）、电平表、RTT、花名册。
 *
 * 边界：这里是「会话层」—— 服务器连接与成员数据的唯一持有者；
 * 聊天/弹幕等 UI 功能通过回调（opts.onRemoteChat）旁路接入，不反向依赖。
 *
 * 生命周期：enterRoom 时 start()，离开/卸载时 teardown()。
 */
import { computed, reactive, ref, type Ref } from 'vue'
import { createLevelMeter, type LevelMeter } from '../lib/level'
import { ensureRemoteAudio } from '../lib/audio'
import { chatFetch, fetchRoster, muteMember, startMic, type MicHandle } from '../lib/sfu'
import { clearPid, loadPid, savePid } from '../lib/session'
import type { ChatItem } from '../lib/sfu'

/** 「加入即听」的初始化发布轨：静音保活，只下行不发声 */
const INITIAL_SOURCE = 'silent' as const
/** 花名册 + 聊天增量 的轮询周期 */
const SYNC_INTERVAL = 2500
/** 电平表采样周期（呼吸圈动画的数据源） */
const LEVEL_INTERVAL = 100
/** 空格 PTT 的长按确认时长；松开早于此 = 单击切换持续开麦 */
const PTT_PRESS_MS = 280

export interface RemoteTrack {
  id: string
  stream: MediaStream
}

export interface RosterUser {
  id: string
  name: string
  owner?: boolean
  /** 订阅心跳申报的开麦状态（成员浮层「开麦中/静音」用） */
  micOn?: boolean
  /** 服务器闭麦账本：该会话当前是否被房主闭麦 */
  muted?: boolean
  joinedMs?: number
}

export interface VoiceSessionOptions {
  room: Ref<string>
  nickname: Ref<string>
  /** 聊天层当前的消息水位（chatFetch 的 since），由会话轮询替聊天层带任意参数 */
  getChatSince: () => number
  /** 2.5s 轮询带回来的聊天增量（转发给房间聊天 composable 的 receive） */
  onRemoteChat: (items: ChatItem[]) => void
}

/** 成员浮层的加入时间文案：yyyy-mm-dd hh:mm */
function joinedText(ms?: number): string {
  if (!ms) return '— 加入'
  const d = new Date(ms)
  const two = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())} ${two(d.getHours())}:${two(d.getMinutes())} 加入`
}

export function useVoiceSession(opts: VoiceSessionOptions) {
  /* ---------- 品牌 ref 群 ---------- */
  const voiceOn = ref(false) // 与 SFU 的信令+媒体连接活着
  const micOn = ref(false) // 上行是真实麦克风（对别人可闻）
  const micBusy = ref(false) // 正在切换（按钮置灰）
  const micErr = ref('')
  /** 通行证失效（服务器重启/在别处重新加入）：UI 提示口径与一般信令失败区分开 */
  const mustRejoin = ref(false)
  /** 我本人被房主闭麦（解除后按钮复活，但不会自动重新开麦） */
  const ownerMuted = ref(false)
  /** 远端 audio 被浏览器拦截（用户点解锁消除） */
  const audioBlocked = ref(false)
  /** 心跳带回被闭麦：立刻切静音轨（不管当前在不在开麦） */
  function onOwnerMute() {
    ownerMuted.value = true
    micLatched.value = false
    pttHolding.value = false
    micOn.value = false
    if (!handle) return
    void handle.setSource('silent').catch(() => {})
  }
  /** 心跳带回解除：只复活按钮，不自动开麦（需要用户自己再按） */
  function onOwnerUnmute() {
    ownerMuted.value = false
  }
  const remotes = ref<RemoteTrack[]>([])
  const levelMe = ref(0)
  const levelRemote = reactive<Record<string, number>>({})
  const ownRtt = ref<number | null>(null)

  let handle: MicHandle | null = null
  let levelTimer: number | undefined
  let audioTimer: number | undefined
  let rttTimer: number | undefined
  let syncTimer: number | undefined

  const remoteMeters = new Map<string, LevelMeter>()

  /* ---------- 通行证 ---------- */
  const pid = ref<string | undefined>(undefined)

  /* ---------- 麦克风开关原语 + PTT（长按说话 / 单击持续开麦） ---------- */
  const micLatched = ref(false) // 单击切换的持续开麦
  const pttHolding = ref(false) // 正在长按
  const micAirborne = computed(() => micOn.value || pttHolding.value)
  const micLabel = computed(() =>
    ownerMuted.value
      ? '被房主闭麦'
      : pttHolding.value
        ? '松开 结束'
        : micLatched.value || micOn.value
          ? '关麦'
          : '按住说话',
  )

  async function engageMic(on: boolean) {
    if (micBusy.value) return
    // 被房主闭麦期间：开麦请求一律拒收（页面按钮已灰，这里拦空格键/代码路径）
    if (on && ownerMuted.value) return
    if (on && !voiceOn.value) {
      await startVoice()
      if (!voiceOn.value) return
    }
    if (!handle) return
    micBusy.value = true
    micErr.value = ''
    try {
      await handle.setSource(on ? 'mic' : 'silent')
      micOn.value = on
    } catch (e: unknown) {
      const eo = e as { name?: string; message?: string }
      micErr.value = on
        ? eo?.name === 'NotAllowedError'
          ? '需要允许麦克风权限'
          : `无法开麦：${eo?.message ?? eo?.name ?? String(e)}`
        : `关麦失败：${eo?.message ?? String(e)}`
    } finally {
      micBusy.value = false
    }
  }

  let pressTimer: number | undefined

  /** 麦按钮按下：280ms 后进入长按 PTT；提前松开 = 单击切换 */
  function pttDown() {
    if (micBusy.value || ownerMuted.value) return
    pressTimer = window.setTimeout(() => {
      pressTimer = undefined
      pttHolding.value = true
      void engageMic(true)
    }, PTT_PRESS_MS)
  }

  function pttUp() {
    if (pressTimer !== undefined) {
      // 短按 = 切换持续开麦
      clearTimeout(pressTimer)
      pressTimer = undefined
      micLatched.value = !micLatched.value
      void engageMic(micLatched.value)
      return
    }
    if (pttHolding.value) {
      pttHolding.value = false
      micLatched.value = false // PTT 结束接管，回到按住说话态
      void engageMic(false) // 松开自动闭麦
    }
  }

  /** 按住中途拖出按钮：与 pttUp 一致按短按切换处理 */
  function pttCancel() {
    if (pressTimer !== undefined) {
      clearTimeout(pressTimer)
      pressTimer = undefined
      micLatched.value = !micLatched.value
      void engageMic(micLatched.value)
    }
  }

  /* 空格 PTT（桌面端）：与按钮共用状态；输入框聚焦时不抢占 */
  function onKeyDown(e: KeyboardEvent) {
    if (e.code !== 'Space' || e.repeat) return
    const tag = (document.activeElement as HTMLElement | null)?.tagName
    if (tag === 'INPUT' || tag === 'TEXTAREA') return
    e.preventDefault()
    if (!pttHolding.value) {
      if (ownerMuted.value) return
      pttHolding.value = true
      micLatched.value = false // 空格 PTT 接管
      void engageMic(true)
    }
  }
  function onKeyUp(e: KeyboardEvent) {
    if (e.code !== 'Space') return
    if (pttHolding.value) {
      pttHolding.value = false
      void engageMic(false)
    }
  }

  /* ---------- SFU 连接 ---------- */
  async function startVoice() {
    if (voiceOn.value || micBusy.value) return
    if (typeof RTCPeerConnection === 'undefined') {
      micErr.value = '此页面不是安全上下文，浏览器禁用了 WebRTC'
      return
    }
    micBusy.value = true
    micErr.value = ''
    mustRejoin.value = false
    pid.value = loadPid(opts.room.value)
    try {
      handle = await startMic({
        room: opts.room.value,
        user: opts.nickname.value || 'me',
        token: pid.value,
        onPid: (p) => {
          pid.value = p
          savePid(opts.room.value, p)
        },
        initial: INITIAL_SOURCE,
        getMicOn: () => micAirborne.value,
        onOwnerMute,
        onOwnerUnmute,
        onRemoteTrack: (stream) => {
          if (!remotes.value.some((r) => r.id === stream.id)) {
            remotes.value.push({ id: stream.id, stream })
          }
          if (stream.id) attachRemoteMeter(stream.id, stream)
          startLevelTick()
        },
        onError: (err) => {
          const msg = err instanceof Error ? err.message : String(err)
          if (msg.includes('session gone') || msg.includes('peer not found')) {
            mustRejoin.value = true
            micErr.value = '连接已失效（服务器重启或在别处加入了），请重新进入'
          } else {
            micErr.value = '信令连接出错，已停止'
          }
          teardown()
        },
      })
      voiceOn.value = true
      startSync()
      startLevelTick()
      startRttPoll()
    } catch (e: unknown) {
      const eo = e as { message?: string; name?: string }
      micErr.value = `无法接入房间音频：${eo?.message ?? eo?.name ?? String(e)}`
    } finally {
      micBusy.value = false
    }
  }

  /** 全部停止：媒体、计时器、电平表；通行证作废（下次进房重新签发） */
  function teardown() {
    handle?.stop()
    handle = null
    voiceOn.value = false
    micOn.value = false
    pttHolding.value = false
    micLatched.value = false
    ownerMuted.value = false
    remotes.value = []
    clearPid(opts.room.value)
    pid.value = undefined
    stopSync()
    for (const t of [levelTimer, audioTimer, rttTimer]) if (t) clearInterval(t)
    levelTimer = audioTimer = rttTimer = undefined
    ownRtt.value = null
    for (const id of [...remoteMeters.keys()]) disposeRemoteMeter(id)
    document.removeEventListener('keydown', onKeyDown)
    document.removeEventListener('keyup', onKeyUp)
  }

  /* ---------- 电平表 + 自动播放拉起 ---------- */
  function attachRemoteMeter(id: string, stream: MediaStream) {
    if (remoteMeters.has(id)) return
    const track = stream.getAudioTracks()[0]
    if (track) remoteMeters.set(id, createLevelMeter(track))
  }

  function disposeRemoteMeter(id: string) {
    remoteMeters.get(id)?.dispose()
    remoteMeters.delete(id)
    delete levelRemote[id]
  }

  function startLevelTick() {
    if (levelTimer) return
    levelTimer = window.setInterval(() => {
      levelMe.value = handle ? Math.min(1, handle.level) : 0
      for (const r of remotes.value) {
        const m = remoteMeters.get(r.id)
        levelRemote[r.id] = m ? m.level() : 0
      }
    }, LEVEL_INTERVAL)
    // 远端 audio 偶发被系统暂停（后台切换/自动播放策略），定期补播
    audioTimer = window.setInterval(async () => {
      audioBlocked.value = await ensureRemoteAudio()
    }, 3000)
  }

  /* 自己到 SFU 的 RTT（成员浮层显示） */
  function startRttPoll() {
    if (rttTimer) return
    const tick = async () => {
      ownRtt.value = handle ? await handle.rtt() : null
    }
    void tick()
    rttTimer = window.setInterval(() => void tick(), 3000)
  }

  /* ---------- 花名册（真名 + 实时增减 + 服务端房主标记） ---------- */
  const rosterUsers = ref<RosterUser[]>([])

  /** 人数优先信服务器；信令刚建立还没拉到时按「远端流数+1」兜底 */
  const memberCount = computed(() => rosterUsers.value.length || remotes.value.length + 1)
  // 房主 = 服务器登记的房间创建者（无标记时回退最早在线者）
  const ownerName = computed(
    () => rosterUsers.value.find((u) => u.owner)?.name ?? rosterUsers.value[0]?.name ?? null,
  )
  const amIOwner = computed(
    () => ownerName.value != null && ownerName.value === opts.nickname.value,
  )

  function remoteName(id: string): string {
    return rosterUsers.value.find((u) => u.id === id)?.name ?? `对端 ${id.slice(0, 6)}`
  }

  function remoteIsOwner(id: string): boolean {
    return rosterUsers.value.find((u) => u.id === id)?.owner === true
  }

  function remoteJoined(id: string): number | undefined {
    return rosterUsers.value.find((u) => u.id === id)?.joinedMs
  }

  /** 该成员心跳申报的开麦状态 */
  function remoteMicOn(id: string): boolean {
    return rosterUsers.value.find((u) => u.id === id)?.micOn === true
  }

  /** 该成员当前是否被房主闭麦 */
  function remoteOwnerMuted(id: string): boolean {
    return rosterUsers.value.find((u) => u.id === id)?.muted === true
  }

  /* ---------- 房主闭麦 ---------- */
  /** 房主发起闭麦/解除（服务端校验身份，双保险：静音 + 心跳带回） */
  async function setRemoteMute(memberId: string, mute: boolean): Promise<string> {
    if (!pid.value) return '连接尚未就绪'
    if (pid.value && !amIOwner.value) return '只有房主可以操作'
    try {
      await muteMember(pid.value, memberId, mute)
      return 'ok'
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      if (msg.includes('not owner')) return '只有房主可以闭麦'
      if (msg.includes('member gone')) return '该成员已离开'
      if (msg.includes('session gone')) {
        mustRejoin.value = true
        micErr.value = '连接已失效（服务器重启或在别处加入了），请重新进入'
        teardown()
        return '连接已失效'
      }
      return `操作失败：${msg}`
    }
  }

  /** 一轮轮询：花名册（顺带清掉已离开的远端电平表）+ 聊天增量 */
  async function pollOnce() {
    try {
      const users = await fetchRoster(opts.room.value)
      rosterUsers.value = users
      const live = new Set(users.map((u) => u.id))
      const gone: string[] = []
      remotes.value = remotes.value.filter((r) => {
        if (live.has(r.id)) return true
        gone.push(r.id)
        return false
      })
      for (const id of gone) disposeRemoteMeter(id)
    } catch {
      /* 下次再拉 */
    }
    // 聊天/弹幕增量（含首次全量历史）；未接入时不拉
    if (voiceOn.value) {
      try {
        opts.onRemoteChat(await chatFetch(opts.room.value, opts.getChatSince()))
      } catch {
        /* 下次再拉 */
      }
    }
  }

  function startSync() {
    if (syncTimer) return
    void pollOnce()
    syncTimer = window.setInterval(pollOnce, SYNC_INTERVAL)
  }

  function stopSync() {
    if (syncTimer) {
      clearInterval(syncTimer)
      syncTimer = undefined
    }
  }

  /* wait: chatFetch 的 since 由 chat 层自己记 — 这里需要 room + lastMsg */
  /* ---------- 生命周期入口 ---------- */
  let started = false

  /** 进房总闸：注册空格 PTT 监听 + 建立连线 + 启动同步（离开/卸载必须 teardown） */
  function start() {
    if (started) return
    started = true
    document.addEventListener('keydown', onKeyDown)
    document.addEventListener('keyup', onKeyUp)
    void startVoice()
  }

  return {
    // 状态
    voiceOn,
    micOn,
    micBusy,
    micErr,
    mustRejoin,
    audioBlocked,
    remotes,
    levelMe,
    levelRemote,
    ownRtt,
    rosterUsers,
    amIOwner,
    ownerName,
    memberCount,
    // 麦克风
    micAirborne,
    micLabel,
    engageMic,
    pttDown,
    pttUp,
    pttCancel,
    // 会话生命周期
    start,
    teardown,
    // 查询
    remoteName,
    remoteIsOwner,
    remoteJoined,
    remoteMicOn,
    remoteOwnerMuted,
    joinedText,
    // 房主闭麦
    ownerMuted,
    setRemoteMute,
    /** 进房前的一次只读探测（闸门判断「房间是否有人」用，不建立连接） */
    probeRoster: () => fetchRoster(opts.room.value),
    getPid: () => pid.value,
  }
}
