<template>
  <div class="room-shell">
    <!-- ============ 加入前：昵称 + 加入码闸门 ============ -->
    <section v-if="!joined" class="join-wrap">
      <header class="join-top">
        <BrandLogo :size="26" />
        <span class="join-room-name">{{ name }}</span>
      </header>

      <div class="join hero">
        <span class="room-avatar rise" :style="avatarStyle">{{ name.slice(0, 1).toUpperCase() }}</span>
        <h1 class="display rise" style="--d: 1">加入「{{ name }}」</h1>
        <p class="lead rise" style="--d: 2">
          起个名字亮个相。<span class="text-3">进来就能听到里面的人在说话；默认关麦，想说话再开麦。</span>
        </p>

        <div class="composer rise" style="--d: 3" :class="{ filled: !!nickname }">
          <input
            ref="nickInput"
            v-model="nickname"
            class="composer-input"
            type="text"
            placeholder="你的昵称"
            autocomplete="off"
            spellcheck="false"
            enterkeyhint="go"
            :maxlength="24"
            @keydown.enter="join"
          />
          <button class="dice" title="随机昵称" @click="rollNickname">
            <svg viewBox="0 0 24 24" width="20" height="20" fill="none">
              <rect x="3.2" y="3.2" width="17.6" height="17.6" rx="4.6" stroke="currentColor" stroke-width="1.7" />
              <circle cx="8.6" cy="8.6" r="1.5" fill="currentColor" />
              <circle cx="15.4" cy="15.4" r="1.5" fill="currentColor" />
              <circle cx="15.4" cy="8.6" r="1.5" fill="currentColor" />
            </svg>
          </button>
        </div>

        <!-- 加入码闸门：房间正在使用中且链接未带码时，必须填对码才能进 -->
        <div v-if="needCode" class="composer rise" style="--d: 3">
          <input
            v-model="codeInput"
            class="composer-input"
            type="text"
            inputmode="numeric"
            pattern="[0-9]*"
            placeholder="加入码 · 向房主要（6 位数字）"
            autocomplete="off"
            spellcheck="false"
            enterkeyhint="go"
            :maxlength="6"
            @input="codeErr = ''"
            @keydown.enter="join"
          />
          <span class="code-mark" :class="{ ok: codeInputOk }">{{ codeInputOk ? '✓' : '' }}</span>
        </div>
        <p v-if="codeErr" class="code-err">{{ codeErr }}</p>

        <button class="cta rise" style="--d: 4" :disabled="!nicknameValid || needCode === null" @click="join">
          <span>进入房间</span>
          <svg viewBox="0 0 24 24" width="17" height="17" fill="none">
            <path d="M5 12h14M13 6l6 6-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>

        <p class="hint text-3 rise" style="--d: 5">每个房间的昵称会记住，下次直接进。</p>
        <p v-if="micErr" class="mic-err">{{ micErr }}</p>
      </div>
    </section>

    <!-- ============ 加入后：会议现场 ============ -->
    <template v-else>
      <header class="topbar">
        <BrandLogo :size="26" />
        <div class="room-title">{{ name }}</div>
        <div class="spacer"></div>
        <button class="chip" @click.stop="toggleMembers">
          <svg viewBox="0 0 24 24" width="15" height="15"><circle cx="9" cy="8" r="3.2" fill="currentColor"/><path d="M3.5 19c.6-3.2 2.8-5 5.5-5s4.9 1.8 5.5 5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/><circle cx="16.5" cy="9" r="2.6" fill="currentColor" opacity=".55"/><path d="M16.5 14.5c2.3.3 3.8 1.9 4.3 4.5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" opacity=".55"/></svg>
          <span class="lbl"><b>{{ memberCount }}</b> 人</span>
          <svg class="info-ico" viewBox="0 0 16 16"><circle cx="8" cy="8" r="6.8" fill="none" stroke="currentColor" stroke-width="1.4"/><circle cx="8" cy="4.9" r="1" fill="currentColor"/><rect x="7.3" y="7" width="1.4" height="4.8" rx="0.7" fill="currentColor"/></svg>
        </button>
        <div class="chip hide-m">{{ timerText }}</div>
        <button class="invite-top" @click="inviteOpen = true">
          <svg viewBox="0 0 24 24" width="15" height="15"><circle cx="10" cy="8" r="3.2" fill="currentColor"/><path d="M4.5 19c.6-3.2 2.8-5 5.5-5s4.9 1.8 5.5 5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/><path d="M18 6v6M15 9h6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
          <span class="lbl">邀请</span>
        </button>
      </header>
      <main class="stage">
        <!-- 我 -->
        <article class="tile" :class="{ speaking: micAirborne }">
          <div class="avatar" :style="[nameColor(nickname), ringStyle(levelMe, true)]">
            {{ avatarText(nickname) }}
            <span v-if="amIOwner" class="owner-badge">房主</span>
          </div>
          <div class="t-name">{{ nickname }}</div>
          <div class="t-status" :class="{ live: micAirborne }" :title="micAirborne ? '开麦中' : '静音'">
            <svg v-if="micAirborne" viewBox="0 0 24 24" width="13" height="13"><rect x="9" y="3" width="6" height="11" rx="3" fill="currentColor"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/></svg>
            <svg v-else viewBox="0 0 24 24" width="13" height="13"><rect x="9" y="3" width="6" height="11" rx="3" fill="none" stroke="currentColor" stroke-width="1.8"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/><path d="M4 4l16 16" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>
          </div>
        </article>

        <!-- 远端成员 -->
        <article v-for="r in remotes" :key="r.id" class="tile" :class="{ speaking: (levelRemote[r.id] ?? 0) > 0.02 }">
          <div class="avatar" :style="[nameColor(remoteName(r.id)), ringStyle(levelRemote[r.id] ?? 0, (levelRemote[r.id] ?? 0) > 0.02)]">
            {{ avatarText(remoteName(r.id)) }}
            <span v-if="remoteIsOwner(r.id)" class="owner-badge">房主</span>
          </div>
          <div class="t-name">{{ remoteName(r.id) }}</div>
          <div class="t-status" :class="{ live: (levelRemote[r.id] ?? 0) > 0.02 }" :title="(levelRemote[r.id] ?? 0) > 0.02 ? '开麦中' : '静音'">
            <svg v-if="(levelRemote[r.id] ?? 0) > 0.02" viewBox="0 0 24 24" width="13" height="13"><rect x="9" y="3" width="6" height="11" rx="3" fill="currentColor"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/></svg>
            <svg v-else viewBox="0 0 24 24" width="13" height="13"><rect x="9" y="3" width="6" height="11" rx="3" fill="none" stroke="currentColor" stroke-width="1.8"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/><path d="M4 4l16 16" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>
          </div>
        </article>
      </main>

      <!-- 远端音频（不可见，负责播放） -->
      <audio v-for="r in remotes" :key="'a' + r.id" :srcObject.prop="r.stream" autoplay playsinline class="remote-audio" />

      <!-- 弹幕层 -->
      <div v-if="danmakuOn" class="danmaku-layer" aria-hidden="true">
        <span v-for="d in danmakuItems" :key="d.id" class="danmaku-item" :style="{ top: d.top + '%' }" @animationend="removeDanmaku(d.id)">{{ d.text }}</span>
      </div>

      <!-- 底部工具栏：腾讯会议式单条（图标+小字，右侧红色结束） -->
      <div class="bottom-bar">
        <div class="dock">
          <button v-if="!dmOpen" class="tb-item" @click="dmOpen = true">
            <svg viewBox="0 0 24 24" width="18" height="18"><rect x="3.2" y="5" width="17.6" height="13" rx="4.2" stroke="currentColor" stroke-width="1.8" fill="none"/><path d="M7.5 10h9M7.5 13.4h5.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>
            <span class="lbl">弹幕</span>
          </button>
          <button
            v-else
            class="tb-item"
            title="收起弹幕输入"
            @click="dmOpen = false"
          >
            <svg viewBox="0 0 24 24" width="18" height="18"><rect x="3.2" y="5" width="17.6" height="13" rx="4.2" stroke="currentColor" stroke-width="1.8" fill="none"/><path d="M7.5 10h9M7.5 13.4h5.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>
            <span class="lbl">弹幕</span>
          </button>

          <button
            class="tb-item primary"
            :class="{ off: micAirborne }"
            :disabled="micBusy"
            @pointerdown="pttDown"
            @pointerup="pttUp"
            @pointerleave="pttCancel"
            @contextmenu.prevent
          >
            <svg viewBox="0 0 24 24" width="18" height="18"><rect x="9" y="3" width="6" height="11" rx="3" fill="currentColor"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/></svg>
            <span class="lbl">{{ micLabel }}</span>
          </button>

          <button class="tb-item" @click="toggleChat">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M12 4.2c-4.2 0-7.6 2.8-7.6 6.3 0 2 1.1 3.8 2.9 5l-.6 3 3.3-1.7c.65.13 1.32.2 2 .2 4.2 0 7.6-2.8 7.6-6.5S16.2 4.2 12 4.2z" stroke="currentColor" stroke-width="1.8" fill="none" stroke-linejoin="round"/></svg>
            <span class="lbl">聊天</span>
            <span v-if="unread > 0" class="fab-badge">{{ unread }}</span>
          </button>

          <span class="tb-divider" aria-hidden="true"></span>

          <button class="tb-item" @click="leaveRoom">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M15 4h3.5A1.5 1.5 0 0 1 20 5.5v13a1.5 1.5 0 0 1-1.5 1.5H15M10 8l-4 4 4 4M6 12h10" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
            <span class="lbl">离开房间</span>
          </button>
          <button v-if="amIOwner" class="end-btn" @click="endRoom">
            <svg viewBox="0 0 24 24" width="15" height="15"><path d="M12 3v8" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/><path d="M6.6 6.6a8 8 0 1 0 10.8 0" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round"/></svg>
            <span class="lbl">结束房间</span>
          </button>
        </div>

        <!-- 弹幕输入条：浮在工具栏上方，不挤占栏内空间 -->
        <div v-if="dmOpen" class="dm-pop">
          <input
            ref="danmakuInputEl"
            v-model="danmakuText"
            type="text"
            placeholder="发条弹幕，回车发射"
            maxlength="60"
            @keydown.enter="danmakuSend"
            @keydown.esc="dmOpen = false"
          />
        </div>

        <p v-if="micErr" class="dock-err">{{ micErr }}</p>
      </div>
      <!-- 成员浮层（右上角 ⓘ 下拉） -->
      <div v-if="membersOpen" class="popover" @click.stop>
        <div class="p-head">
          <div class="p-title">成员 <small>{{ memberCount }} 人在线</small></div>
          <button v-if="amIOwner && remotes.length" class="mute-all" @click="muteAllDemo">一键闭麦</button>
        </div>
        <div class="m-row">
          <div class="m-avatar">{{ nickname.slice(0, 1) || '我' }}</div>
          <div class="m-main">
            <div class="m-name"><span v-if="amIOwner" class="owner-tag">房主</span>{{ nickname }}</div>
            <div class="m-sub">{{ joinedText(joinedAt) }} · <span class="mic-on">开麦中</span></div>
          </div>
          <div class="m-ping"><div class="ms"><span class="pdot" :class="pingDot(ownRtt)"></span>{{ ownRtt ?? '—' }}{{ ownRtt ? 'ms' : '' }}</div></div>
        </div>
        <div v-for="r in remotes" :key="'m' + r.id" class="m-row">
          <div class="m-avatar alt">{{ remoteName(r.id).slice(0, 1) }}</div>
          <div class="m-main">
            <div class="m-name"><span v-if="remoteIsOwner(r.id)" class="owner-tag">房主</span>{{ remoteName(r.id) }}</div>
            <div class="m-sub">{{ joinedText(remoteJoined(r.id)) }} · {{ (levelRemote[r.id] ?? 0) > 0.02 ? '开麦中' : '静音' }}</div>
          </div>
          <button v-if="amIOwner" class="mute-btn" @click="muteDemo(remoteName(r.id))">
            <svg viewBox="0 0 24 24" width="12" height="12"><rect x="9" y="3" width="6" height="11" rx="3" fill="currentColor"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/><path d="M4 4l16 16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
            闭麦
          </button>
          <div v-else class="m-ping"><div class="ms">—</div></div>
        </div>
      </div>

      <!-- 邀请弹层 -->
      <div v-if="inviteOpen" class="sheet-backdrop" @click.self="inviteOpen = false">
        <div class="sheet">
          <button class="x" @click="inviteOpen = false">✕</button>
          <h3>邀请加入「{{ name }}」</h3>
          <div class="code-row">
            <span class="code">{{ code }}</span>
            <button class="copy-mini" :class="{ done: codeCopied }" @click="copyCode">{{ codeCopied ? '✓ 已复制' : '复制码' }}</button>
          </div>
          <div class="link-row"><span>{{ displayUrl }}</span></div>
          <button class="go" :class="{ done: textCopied }" @click="copyText">{{ textCopied ? '✓ 已复制，去粘贴吧' : '复制邀请语，发到群里' }}</button>
        </div>
      </div>

      <!-- 聊天抽屉（PC 右侧 / 手机底部半屏） -->
      <aside class="chat-panel" :class="{ open: chatOpen }">
        <div class="chat-head">会议聊天<small>{{ memberCount }} 人</small><button class="chat-x" @click="chatOpen = false">✕</button></div>
        <div class="chat-body" ref="chatBodyEl">
          <div v-if="!messages.length" class="msg-sys">还没有消息，说句打个招呼吧</div>
          <div v-for="(m, i) in messages" :key="i" class="msg" :class="{ mine: m.mine }">
            <div class="mini-av" :class="{ alt: !m.mine }">{{ m.name.slice(0, 1) }}</div>
            <div>
              <div class="bubble"><div v-if="!m.mine" class="b-name">{{ m.name }}</div>{{ m.text }}</div>
            </div>
          </div>
        </div>
        <div class="chat-input">
          <input v-model="chatText" type="text" placeholder="发消息…" maxlength="200" @keydown.enter="chatSendMsg" />
          <button @click="chatSendMsg">发送</button>
        </div>
      </aside>

      <div v-if="toast" class="toast">{{ toast }}</div>

      <!-- 声音被浏览器拦截的兜底解锁 -->
      <div v-if="audioBlocked" class="audio-unlock" @click="unlockManual">🔊 点我开启声音</div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch, type Ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import BrandLogo from '../components/BrandLogo.vue'
import { getNickname, saveRoom } from '../lib/rooms'
import { randomNickname } from '../lib/random'
import { joinCode, normalizeCode, shareText } from '../lib/invite'
import { chatFetch, chatSend, startMic, fetchRoster, type MicHandle } from '../lib/sfu'
import { createLevelMeter, type LevelMeter } from '../lib/level'

const route = useRoute()
const router = useRouter()
const name = computed(() => decodeURIComponent(String(route.params.name ?? 'UNNAMED')))

/* ---------- 加入流程 ---------- */
const nickname = ref('')
const joined = ref(false)
const nickInput = ref<HTMLInputElement | null>(null)
const nicknameValid = computed(() => nickname.value.trim().length > 0)

/* ---------- 加入码闸门（软门槛：链接带码免填；房间使用中才拦） ---------- */
const codeInput = ref('')
const codeErr = ref('')
const needCode = ref<boolean | null>(null)
const urlCode = new URLSearchParams(location.search).get('c') ?? ''
const codeVerified = ref(normalizeCode(urlCode) === normalizeCode(joinCode(name.value)))
const code = computed(() => joinCode(name.value))
const codeInputOk = computed(() => normalizeCode(codeInput.value) === normalizeCode(code.value))

function rollNickname() {
  nickname.value = randomNickname()
  nickInput.value?.focus()
}

function join() {
  const nick = nickname.value.trim()
  if (!nicknameValid.value) return
  if (needCode.value === null) return
  if (needCode.value && !codeVerified.value && !codeInputOk.value) {
    codeErr.value = '加入码不对，向房主确认一下'
    return
  }
  saveRoom(name.value, nick)
  enterRoom()
}

/** 进入会议现场（join 与本机记住昵称的自动进共用） */
function enterRoom() {
  joined.value = true
  joinedAt = Date.now()
  startRoomTimers()
  // 用户手势内解锁媒体播放通道（iOS/安卓自动播放拦截）
  unlockPlayback()
  void startVoice()
}

/* ---------- 顶栏计时 ---------- */
let joinedAt = 0
let clockTimer: number | undefined
const timerText = ref('00:00')

function startRoomTimers() {
  if (clockTimer) return
  clockTimer = window.setInterval(() => {
    const s = Math.floor((Date.now() - joinedAt) / 1000)
    const h = Math.floor(s / 3600)
    const m = Math.floor((s % 3600) / 60)
    const sec = s % 60
    const two = (n: number) => String(n).padStart(2, '0')
    timerText.value = h > 0 ? `${h}:${two(m)}:${two(sec)}` : `${two(m)}:${two(sec)}`
  }, 1000)
}

/* ---------- 媒体播放解锁 ---------- */
function unlockPlayback() {
  const a = document.createElement('audio')
  a.setAttribute('playsinline', 'true')
  a.src = 'data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQAAAAA='
  void a.play().catch(() => {})
}

async function ensureRemoteAudio() {
  await nextTick()
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
  audioBlocked.value = blocked
}

function unlockManual() {
  unlockPlayback()
  void ensureRemoteAudio().then(() => (audioBlocked.value = false))
}

/* ---------- 语音（加入即听；麦克风只管上行开/关） ---------- */
const voiceOn = ref(false)
const micOn = ref(false)
const micBusy = ref(false)
const micErr = ref('')
const remotes = ref<Array<{ id: string; stream: MediaStream }>>([])
const levelMe = ref(0)
const levelRemote = reactive<Record<string, number>>({})
const remoteMeters = new Map<string, LevelMeter>()
const audioBlocked = ref(false)
let micHandle: MicHandle | null = null
let levelTimer: number | undefined
let rttTimer: number | undefined
const ownRtt = ref<number | null>(null)

async function startVoice() {
  if (voiceOn.value || micBusy.value) return
  if (typeof RTCPeerConnection === 'undefined') {
    micErr.value = '此页面不是安全上下文，浏览器禁用了 WebRTC'
    return
  }
  micBusy.value = true
  micErr.value = ''
  try {
    micHandle = await startMic({
      room: name.value,
      user: nickname.value || 'me',
      initial: 'silent',
      onRemoteTrack: (stream) => {
        if (!remotes.value.some((r) => r.id === stream.id)) {
          remotes.value.push({ id: stream.id, stream })
        }
        if (stream.id) attachRemoteMeter(stream.id, stream)
        startLevelTick()
      },
      onError: () => {
        micErr.value = '信令连接出错，已停止'
        teardownVoice()
      },
    })
    voiceOn.value = true
    startRosterPoll()
    startLevelTick()
    startRttPoll()
  } catch (e: any) {
    micErr.value = `无法接入房间音频：${e?.message ?? e?.name ?? String(e)}`
  } finally {
    micBusy.value = false
  }
}

function teardownVoice() {
  micHandle?.stop()
  micHandle = null
  voiceOn.value = false
  micOn.value = false
  pttHolding.value = false
  micLatched.value = false
  remotes.value = []
  stopRosterPoll()
  if (levelTimer) { clearInterval(levelTimer); levelTimer = undefined }
  if (rttTimer) { clearInterval(rttTimer); rttTimer = undefined }
  ownRtt.value = null
  for (const id of [...remoteMeters.keys()]) disposeRemoteMeter(id)
}

/* ---------- 麦克风开/关原语 + 按住说话（PTT） ---------- */
async function engageMic(on: boolean) {
  if (micBusy.value) return
  if (on && !voiceOn.value) {
    await startVoice()
    if (!voiceOn.value) return
  }
  if (!micHandle) return
  micBusy.value = true
  micErr.value = ''
  try {
    await micHandle.setSource(on ? 'mic' : 'silent')
    micOn.value = on
  } catch (e: any) {
    micErr.value = on
      ? e?.name === 'NotAllowedError'
        ? '需要允许麦克风权限'
        : `无法开麦：${e?.message ?? e?.name ?? String(e)}`
      : `关麦失败：${e?.message ?? String(e)}`
  } finally {
    micBusy.value = false
  }
}

const micLatched = ref(false) // 单击切换的持续开麦
const pttHolding = ref(false) // 正在长按（按钮或空格）
const micAirborne = computed(() => micOn.value || pttHolding.value)
const micLabel = computed(() => (pttHolding.value ? '松开 结束' : micLatched.value || micOn.value ? '关麦' : '按住说话'))

let pressTimer: number | undefined

function pttDown() {
  if (micBusy.value) return
  pressTimer = window.setTimeout(() => {
    pressTimer = undefined
    pttHolding.value = true
    void engageMic(true)
  }, 280)
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

function pttCancel() {
  if (pressTimer !== undefined) {
    clearTimeout(pressTimer)
    pressTimer = undefined
    micLatched.value = !micLatched.value
    void engageMic(micLatched.value)
  }
}

window.addEventListener('keydown', (e) => {
  if (!joined.value || e.code !== 'Space' || e.repeat) return
  const tag = (document.activeElement as HTMLElement | null)?.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA') return
  e.preventDefault()
  if (!pttHolding.value) {
    pttHolding.value = true
    micLatched.value = false // 空格 PTT 接管
    void engageMic(true)
  }
})
window.addEventListener('keyup', (e) => {
  if (e.code !== 'Space') return
  if (pttHolding.value) {
    pttHolding.value = false
    void engageMic(false)
  }
})
/* ---------- 电平表（呼吸圈数据源）+ 自动播放拉起 ---------- */
function startLevelTick() {
  if (levelTimer) return
  levelTimer = window.setInterval(() => {
    levelMe.value = micHandle ? Math.min(1, micHandle.level) : 0
    for (const r of remotes.value) {
      const m = remoteMeters.get(r.id)
      levelRemote[r.id] = m ? m.level() : 0
    }
  }, 100)
  window.setInterval(() => void ensureRemoteAudio(), 3000)
}

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

/* 自己到 SFU 的 RTT（成员浮层显示） */
function startRttPoll() {
  if (rttTimer) return
  const tick = async () => {
    ownRtt.value = micHandle ? await micHandle.rtt() : null
  }
  void tick()
  rttTimer = window.setInterval(() => void tick(), 3000)
}

/* ---------- 花名册（真名 + 实时增减 + 服务端房主标记） ---------- */
const rosterUsers = ref<Array<{ id: string; name: string; owner?: boolean; joinedMs?: number }>>([])
let rosterTimer: number | undefined

const memberCount = computed(() => rosterUsers.value.length || remotes.value.length + 1)
// 房主 = 服务器登记的房间创建者（无标记时回退最早在线者）
const ownerName = computed(
  () => rosterUsers.value.find((u) => u.owner)?.name ?? rosterUsers.value[0]?.name ?? null,
)
const amIOwner = computed(() => ownerName.value != null && ownerName.value === nickname.value)

function remoteName(id: string): string {
  return rosterUsers.value.find((u) => u.id === id)?.name ?? `对端 ${id.slice(0, 6)}`
}

function remoteIsOwner(id: string): boolean {
  return rosterUsers.value.find((u) => u.id === id)?.owner === true
}

/** 成员浮层里的加入时间：yyyy-mm-dd hh:mm */
function joinedText(ms?: number): string {
  if (!ms) return '— 加入'
  const d = new Date(ms)
  const two = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())} ${two(d.getHours())}:${two(d.getMinutes())} 加入`
}

function remoteJoined(id: string): number | undefined {
  return rosterUsers.value.find((u) => u.id === id)?.joinedMs
}

async function pollRoster() {
  try {
    const users = await fetchRoster(name.value)
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
  // 聊天/弹幕增量（含首次全量历史）
  if (voiceOn.value) {
    try {
      mergeChat(await chatFetch(name.value, lastMsgId))
    } catch {
      /* 下次再拉 */
    }
  }
}

function startRosterPoll() {
  if (rosterTimer) return
  void pollRoster()
  rosterTimer = window.setInterval(pollRoster, 2500)
}

function stopRosterPoll() {
  if (rosterTimer) {
    clearInterval(rosterTimer)
    rosterTimer = undefined
  }
}
/* ---------- UI 状态：浮层 / 邀请 / 聊天 / 弹幕 / 提示 ---------- */
const membersOpen = ref(false)
const inviteOpen = ref(false)
const chatOpen = ref(false)
const chatText = ref('')
const chatBodyEl = ref<HTMLElement | null>(null)
const unread = ref(0)
const toast = ref('')
let toastTimer: number | undefined

interface ChatMsg { id?: number; mine: boolean; name: string; text: string }
const messages = ref<ChatMsg[]>([])
let lastMsgId = 0

function showToast(msg: string) {
  toast.value = msg
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = window.setTimeout(() => (toast.value = ''), 2200)
}

function toggleMembers() {
  membersOpen.value = !membersOpen.value
}
/** 点击 chip 或浮层以外区域才关闭（比 stopPropagation 时序更可靠） */
const closeOnOutside = (e: Event) => {
  if (!(e.target as HTMLElement)?.closest('.chip, .popover')) membersOpen.value = false
}
document.addEventListener('click', closeOnOutside)

function toggleChat() {
  chatOpen.value = !chatOpen.value
  if (chatOpen.value) {
    unread.value = 0
    void nextTick(() => scrollChat())
  }
}

function scrollChat() {
  const el = chatBodyEl.value
  if (el) el.scrollTop = el.scrollHeight
}

/** 发送 = 服务器入房缓冲；本端立即上屏 + 飞弹幕 */
async function sendChat(text: string) {
  const t = text.trim()
  if (!t || !voiceOn.value) return
  try {
    await chatSend(name.value, nickname.value || '我', t)
    // 轮询会带上自己的消息？不会：本端先上屏，lastMsgId 已推进，轮询不会再发
    appendChat({ id: undefined, mine: true, name: nickname.value || '我', text: t })
    if (danmakuOn.value) pushDanmaku(t)
  } catch {
    showToast('消息没发出去，稍后再试试')
  }
}

/** 聊天抽屉回车/发送 */
function chatSendMsg() {
  const t = chatText.value
  if (!t.trim()) return
  chatText.value = ''
  void nextTick(() => scrollChat())
  void sendChat(t)
}

/** 增量合并服务器消息（轮询 cofetch 用） */
function mergeChat(items: Array<{ id: number; user: string; text: string }>) {
  let grew = false
  for (const m of items) {
    if (m.id <= lastMsgId) continue
    lastMsgId = m.id
    const mine = m.user === (nickname.value || '我')
    // 自己发的已在发送时上屏过，这里只补远端消息
    if (mine) continue
    appendChat({ id: m.id, mine: false, name: m.user, text: m.text })
    if (danmakuOn.value) pushDanmaku(m.text) // 跨端弹幕/聊天互见
    if (!chatOpen.value) unread.value += 1
    grew = true
  }
  if (grew && chatOpen.value) void nextTick(() => scrollChat())
}

function appendChat(m: ChatMsg) {
  messages.value.push(m)
  if (messages.value.length > 400) messages.value.shift()
}

/* 弹幕：显示层默认常开（谁发的都飞过屏幕），💬 只负责打开/收起输入条 */
const danmakuOn = ref(true)
const dmOpen = ref(false)
const danmakuItems = ref<Array<{ id: number; text: string; top: number }>>([])
const danmakuText = ref('')
const danmakuInputEl = ref<HTMLInputElement | null>(null)
let danmakuSeq = 0

/** 开输入条时自动聚焦，敲完直接回车发送 */
watch(dmOpen, (open) => {
  if (open) void nextTick(() => danmakuInputEl.value?.focus())
})

function danmakuSend() {
  const t = danmakuText.value.trim()
  if (!t) return
  danmakuText.value = ''
  void sendChat(t) // 服务器入账 + 本端上屏 + 立即飞弹幕
}
function pushDanmaku(text: string) {
  if (!danmakuOn.value) return
  const id = ++danmakuSeq
  const top = 12 + Math.floor(((id * 37) % 56)) // 12%–68% 伪随机高度
  danmakuItems.value.push({ id, text, top })
  if (danmakuItems.value.length > 12) danmakuItems.value.shift()
}

function removeDanmaku(id: number) {
  danmakuItems.value = danmakuItems.value.filter((d) => d.id !== id)
}

/* 房主管控（前端占位：真正闭麦需服务端停发音频） */
function muteDemo(name2: string) {
  showToast(`「${name2}」闭麦需要服务端支持，即将上线`)
}
function muteAllDemo() {
  showToast('一键闭麦需要服务端支持，即将上线')
}

/* ---------- 离开 / 结束 ---------- */
function leaveRoom() {
  teardownVoice()
  router.push('/')
}

function endRoom() {
  if (confirm('结束房间后所有成员都将被移出，确定？')) {
    // 前端先行版本：解散语义由服务端上线后补全，当前先退出自己
    leaveRoom()
  }
}
/* ---------- 样式辅助 ---------- */
/** 头像底色：按昵称哈希从 4 组低饱和渐变（白蓝系统邻近色）里取，整场色彩克制 */
const AVATAR_GRADS = [
  'linear-gradient(135deg, #6ea4ff 0%, #3d7bff 100%)', // 晴蓝
  'linear-gradient(135deg, #5cc6d4 0%, #2ba0b8 100%)', // 湖青
  'linear-gradient(135deg, #6cc9a4 0%, #3aa580 100%)', // 苔绿
  'linear-gradient(135deg, #9aa6e8 0%, #7583cf 100%)', // 雾紫
]
function nameColor(s: string): string {
  let h = 0
  for (const ch of s) h = (h * 31 + ch.codePointAt(0)!) >>> 0
  return `background: ${AVATAR_GRADS[h % AVATAR_GRADS.length]}`
}

/** 头像取字：中文/混排取前 2 个字符，纯英文取词首字母（John Smith → JS） */
function avatarText(s: string): string {
  const t = (s || '我').trim()
  if (!t) return '我'
  if (/^[A-Za-z][A-Za-z\s.'-]*$/.test(t)) {
    const parts = t.split(/[\s.'-]+/).filter(Boolean)
    return parts.length >= 2 ? (parts[0][0] + parts[1][0]).toUpperCase() : t.slice(0, 2).toUpperCase()
  }
  return [...t].slice(0, 2).join('')
}

const avatarStyle = computed(() => nameColor(name.value))

/** 说话呼吸圈：强度随真实电平（me 常亮基线，remote 仅有声时亮） */
function ringStyle(le: number, active: boolean): string {
  if (!active) return ''
  const k = 0.18 + Math.min(1, le) * 0.82
  return [
    'box-shadow:',
    `0 0 0 3px #fff,`,
    `0 0 0 ${(3 + 5 * k).toFixed(1)}px rgba(31,107,255,${(0.22 + 0.4 * k).toFixed(2)}),`,
    `0 0 ${(10 + 22 * k).toFixed(0)}px ${(2 + 7 * k).toFixed(0)}px rgba(31,107,255,${(0.1 + 0.25 * k).toFixed(2)})`,
  ].join(' ')
}

function pingDot(ms: number | null): string {
  if (ms == null) return 'o'
  return ms < 80 ? 'g' : ms < 200 ? 'o' : 'r'
}

/* ---------- 复制 ---------- */
const shareUrl = computed(() => {
  const u = new URL(location.href)
  u.searchParams.set('c', code.value)
  return u.toString()
})
const displayUrl = computed(() => `${location.host}${decodeURIComponent(location.pathname)}?c=${code.value}`)
const textCopied = ref(false)
const codeCopied = ref(false)

async function doCopy(text: string, flag: Ref<boolean>) {
  try {
    await navigator.clipboard.writeText(text)
    flag.value = true
    setTimeout(() => (flag.value = false), 1600)
  } catch { /* 剪贴板权限被拒时静默 */ }
}
const copyCode = () => doCopy(code.value, codeCopied)
const copyText = () => doCopy(shareText(name.value, shareUrl.value, code.value), textCopied)

/* ---------- 生命周期 ---------- */
onMounted(async () => {
  const saved = getNickname(name.value)
  if (saved) {
    nickname.value = saved
    enterRoom() // 本机来过：自动进（加入即听）
    return
  }
  if (codeVerified.value) {
    needCode.value = false
    return
  }
  try {
    const users = await fetchRoster(name.value)
    needCode.value = users.length > 0
  } catch {
    needCode.value = false // 探测失败宁可放行
  }
})

onBeforeUnmount(() => {
  teardownVoice()
  if (clockTimer) { clearInterval(clockTimer); clockTimer = undefined }
  if (toastTimer) clearTimeout(toastTimer)
  document.removeEventListener('click', closeOnOutside)
})
</script>

<style scoped>
/* ---------- 壳与顶栏 ---------- */
.room-shell { min-height: 100vh; display: flex; flex-direction: column; }

.topbar { display: flex; align-items: center; gap: 10px; padding: 12px 18px; }
.room-title { font-weight: 750; font-size: 16px; }
.spacer { flex: 1; }

.chip {
  height: 36px; padding: 0 12px; border-radius: 999px;
  border: 1px solid var(--border); background: var(--surface);
  color: var(--text-2); font-size: 12.5px;
  display: inline-flex; align-items: center; gap: 6px;
  cursor: pointer; box-shadow: 0 2px 8px rgba(23, 50, 93, 0.06);
  transition: border-color 0.2s, color 0.2s;
}
.chip:hover { border-color: var(--border-hover); color: var(--text); }
.chip b { color: var(--blue); }
.chip .info-ico { width: 13px; height: 13px; flex-shrink: 0; color: var(--text-3); opacity: 0.7; margin-left: 3px; }

.invite-top {
  height: 36px; padding: 0 16px; border: 0; border-radius: 999px;
  background: var(--blue); color: #fff; font-size: 13px; font-weight: 700;
  display: inline-flex; align-items: center; gap: 6px; justify-content: center;
  cursor: pointer; box-shadow: 0 4px 14px rgba(31, 107, 255, 0.3);
  transition: filter 0.2s, transform 0.15s var(--ease-out);
}
.invite-top:hover { filter: brightness(1.1); }
.invite-top:active { transform: scale(0.97); }

/* ---------- 加入前 ---------- */
.join-wrap { width: min(760px, 100%); margin: 0 auto; padding: 20px clamp(16px, 4vw, 32px) 72px; }
.join-top { display: flex; align-items: center; gap: 10px; }
.join-room-name { font-weight: 750; font-size: 16px; }

.join { margin-top: clamp(40px, 10vw, 88px); display: flex; flex-direction: column; }
.room-avatar {
  display: flex; align-items: center; justify-content: center;
  width: 64px; height: 64px; border-radius: 20px;
  color: rgba(255, 255, 255, 0.92); font-size: 24px; font-weight: 800;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.3); box-shadow: var(--shadow-glow);
}
.display { margin: 20px 0 0; font-size: clamp(26px, 5.5vw, 42px); font-weight: 800; letter-spacing: -0.03em; line-height: 1.14; }
.lead { margin: 14px 0 0; max-width: 30em; font-size: clamp(14px, 2.4vw, 15.5px); line-height: 1.7; }

.composer {
  margin-top: clamp(20px, 4vw, 30px);
  display: flex; align-items: center; gap: 4px;
  width: min(420px, 100%); padding: 7px 7px 7px 20px;
  border-radius: 999px; background: var(--surface); border: 1px solid var(--border);
  transition: border-color 0.25s, background 0.25s;
}
.composer.filled { border-color: var(--border-hover); background: var(--surface-strong); }
.composer-input { flex: 1; min-width: 0; height: 44px; border: 0; outline: 0; background: transparent; color: var(--text); font-size: 16px; font-weight: 500; }
.dice {
  width: 40px; height: 40px; border: 0; border-radius: 50%; background: transparent;
  color: var(--text-3); cursor: pointer;
  display: inline-flex; align-items: center; justify-content: center;
  transition: transform 0.35s var(--ease-spring), color 0.2s, background 0.2s;
}
.dice:hover { color: var(--text); background: var(--surface-strong); transform: rotate(90deg) scale(1.06); }

.code-mark { flex-shrink: 0; width: 30px; margin-right: 8px; text-align: center; font-size: 16px; font-weight: 800; color: var(--ok); opacity: 0; transition: opacity 0.2s; }
.code-mark.ok { opacity: 1; }
.code-err { width: min(420px, 100%); margin: 8px 0 -10px; color: var(--danger); font-size: 12.5px; }

.cta {
  margin-top: 18px; display: inline-flex; align-items: center; gap: 8px;
  height: 50px; padding: 0 24px; border: 0; border-radius: 999px;
  background: var(--brand-grad); color: #fff;
  font-size: 15px; font-weight: 750; cursor: pointer;
  align-self: flex-start; position: relative; overflow: hidden;
  transition: filter 0.2s, transform 0.15s var(--ease-out), opacity 0.2s;
}
.cta:hover:not(:disabled) { filter: brightness(1.1); }
.cta:active:not(:disabled) { transform: scale(0.97); }
.cta:disabled { opacity: 0.35; cursor: not-allowed; }
.hint { margin: 14px 0 0; font-size: 12.5px; }
.mic-err { margin: 10px 0 0; color: var(--danger); font-size: 12px; }
/* ---------- 舞台瓷片 ---------- */
.stage {
  flex: 1; width: min(980px, 100%); margin: 0 auto;
  padding: 8px 18px 150px;
  display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  gap: 14px; align-content: start;
}
.tile {
  background: var(--surface); border: 1px solid var(--border);
  border-radius: 22px; padding: 22px 16px 18px;
  display: flex; flex-direction: column; align-items: center; gap: 10px;
  box-shadow: 0 4px 18px rgba(23, 50, 93, 0.05);
}
.avatar {
  position: relative;
  width: 92px; height: 92px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  color: #fff; font-size: 28px; font-weight: 750; letter-spacing: 1px;
  transition: box-shadow 0.2s;
}
/* 房主徽章：头像右上角 45°（飞书徽章规范位） */
.owner-badge {
  position: absolute; top: -5px; right: -14px;
  padding: 1px 6px; border-radius: 999px;
  background: var(--blue); color: #fff;
  font-size: 10px; font-weight: 700; letter-spacing: 0;
  border: 2px solid var(--surface);
  box-shadow: 0 2px 6px rgba(23, 50, 93, 0.18);
}
.t-name {
  display: inline-flex; align-items: center; justify-content: center; gap: 6px;
  font-size: 15px; font-weight: 750; max-width: 100%;
}
/* 麦状态：图标（灰=静音，绿=开麦/说话中） */
.t-status { color: var(--text-3); display: inline-flex; }
.t-status svg { display: block; }
.t-status.live { color: var(--ok); }
.owner-tag {
  font-size: 10px; font-weight: 700; color: var(--blue);
  border: 1px solid rgba(31, 107, 255, 0.4); border-radius: 5px; padding: 1px 5px;
}
.remote-audio { display: none; }

/* ---------- 底部工具栏（腾讯会议式单条） ---------- */
.bottom-bar {
  position: fixed; left: 0; right: 0; bottom: 20px; z-index: 20;
  display: flex; justify-content: center;
  padding: 0 12px;
  pointer-events: none;
}
.bottom-bar > * { pointer-events: auto; }
.dock {
  display: flex; align-items: center; gap: 2px;
  padding: 7px 10px; border-radius: 22px;
  background: var(--surface); border: 1px solid var(--border);
  box-shadow: 0 12px 40px rgba(23, 50, 93, 0.16);
}
/* 工具栏项：图标在上 + 10.5px 小字在下 */
.tb-item {
  position: relative;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 3px; min-width: 62px; height: 46px; padding: 0 12px;
  border: 0; border-radius: 14px; background: transparent;
  color: var(--text-2); font-size: 10.5px; font-weight: 600;
  cursor: pointer;
  transition: background 0.15s, color 0.2s;
  touch-action: none; user-select: none; -webkit-user-select: none;
}
.tb-item:hover { background: var(--surface-strong); color: var(--text); }
.tb-item svg { flex-shrink: 0; }
.tb-item.primary { color: var(--blue); }
.tb-item.primary:hover { color: var(--blue); background: rgba(31, 107, 255, 0.08); }
.tb-item.primary.off { color: var(--danger); }
.tb-item:disabled { opacity: 0.5; cursor: wait; }
.tb-divider { width: 1px; height: 24px; background: var(--border); margin: 0 6px; flex-shrink: 0; }
.end-btn {
  height: 38px; padding: 0 16px; margin-left: 4px; border: 0; border-radius: 999px;
  background: var(--danger); color: #fff; font-size: 13px; font-weight: 700;
  display: inline-flex; align-items: center; gap: 6px;
  cursor: pointer; box-shadow: 0 6px 18px rgba(229, 72, 77, 0.32);
  transition: filter 0.2s, transform 0.15s var(--ease-out);
}
.end-btn:hover { filter: brightness(1.08); }
.end-btn:active { transform: scale(0.97); }
.fab-badge {
  position: absolute; top: 4px; right: 10px;
  min-width: 16px; height: 16px; padding: 0 4px;
  border-radius: 999px; background: var(--danger); color: #fff;
  font-size: 10px; font-weight: 800;
  display: flex; align-items: center; justify-content: center;
}
.dock-err {
  position: absolute; left: 50%; bottom: calc(100% + 8px); transform: translateX(-50%);
  width: max-content; max-width: 92vw;
  color: var(--danger); font-size: 12px; text-align: center;
}
/* 弹幕输入条：浮在工具栏上方 */
.dm-pop {
  position: absolute; bottom: calc(100% + 10px); left: 50%; transform: translateX(-50%);
  display: flex; align-items: center; gap: 8px;
  width: min(340px, calc(100vw - 32px)); height: 46px; padding: 0 16px;
  background: var(--surface); border: 1px solid var(--border); border-radius: 999px;
  box-shadow: 0 12px 40px rgba(23, 50, 93, 0.18);
  animation: pop-in 0.2s var(--ease-out);
}
@keyframes pop-in { from { opacity: 0; transform: translate(-50%, 8px); } }
.dm-pop input {
  flex: 1; min-width: 0; height: 100%;
  border: 0; outline: 0; background: transparent;
  font-size: 13.5px; color: var(--text);
}
.dm-pop input::placeholder { color: var(--text-3); }

/* ---------- 弹幕 ---------- */
.danmaku-layer { position: fixed; inset: 0; z-index: 15; pointer-events: none; overflow: hidden; }
.danmaku-item {
  position: absolute; left: 100%;
  white-space: nowrap;
  color: var(--text);
  font-size: 14px; font-weight: 500;
  text-shadow: 0 1px 6px rgba(255, 255, 255, 0.9), 0 0 2px rgba(255, 255, 255, 0.9);
  animation: fly 9s linear forwards;
}
@keyframes fly { to { transform: translateX(calc(-100vw - 100%)); } }
/* ---------- 成员浮层 ---------- */
.popover {
  position: fixed; top: 62px; right: 16px; z-index: 40;
  width: 300px; padding: 6px 10px;
  background: var(--surface); border: 1px solid var(--border);
  border-radius: 16px; box-shadow: 0 16px 48px rgba(18, 32, 60, 0.18);
}
.p-head { display: flex; align-items: center; justify-content: space-between; }
.p-title { padding: 8px 6px 6px; font-size: 13px; font-weight: 750; border-bottom: 1px solid rgba(23, 50, 93, 0.06); flex: 1; }
.p-title small { color: var(--text-3); font-weight: 500; margin-left: 6px; }
.m-row { display: flex; align-items: center; gap: 10px; padding: 9px 4px; }
.m-avatar {
  width: 32px; height: 32px; border-radius: 50%; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  color: #fff; font-size: 13px; font-weight: 750;
  background: linear-gradient(135deg, #3f8cff, #1f6bff);
}
.m-avatar.alt { background: linear-gradient(135deg, #12b886, #0ca678); }
.m-main { flex: 1; min-width: 0; }
.m-name { display: flex; align-items: center; gap: 6px; font-size: 13.5px; font-weight: 750; }
.m-sub { font-size: 11px; color: var(--text-3); margin-top: 2px; }
.m-sub .mic-on { color: var(--blue); font-weight: 600; }
.m-ping { text-align: right; flex-shrink: 0; }
.m-ping .ms { font-size: 12px; font-weight: 700; font-variant-numeric: tabular-nums; }
.pdot { display: inline-block; width: 6px; height: 6px; border-radius: 50%; margin-right: 4px; }
.pdot.g { background: var(--ok); }
.pdot.o { background: #f5a623; }
.pdot.r { background: var(--danger); }
.mute-all { border: 0; background: none; cursor: pointer; font-size: 12px; font-weight: 700; color: var(--danger); padding: 4px 8px; border-radius: 8px; }
.mute-all:hover { background: rgba(229, 72, 77, 0.08); }
.mute-btn {
  flex-shrink: 0; height: 28px; padding: 0 10px;
  border: 1px solid var(--border); border-radius: 999px;
  background: var(--surface); color: var(--text-2);
  font-size: 11.5px; font-weight: 700; cursor: pointer;
  display: inline-flex; align-items: center; gap: 5px;
}
.mute-btn:hover { border-color: rgba(229, 72, 77, 0.5); color: var(--danger); }
/* ---------- 邀请弹层 ---------- */
.sheet-backdrop {
  position: fixed; inset: 0; background: rgba(18, 32, 60, 0.4);
  display: flex; align-items: center; justify-content: center; z-index: 30;
}
.sheet {
  width: min(420px, calc(100vw - 32px));
  background: var(--surface); border-radius: 24px; padding: 24px;
  box-shadow: 0 24px 70px rgba(18, 32, 60, 0.3);
}
.sheet h3 { font-size: 17px; margin-bottom: 4px; }
.sheet .sub { font-size: 12.5px; color: var(--text-3); margin-bottom: 16px; }
.sheet .x { float: right; border: 0; background: none; font-size: 18px; color: var(--text-3); cursor: pointer; }
.code-row { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; flex-wrap: wrap; }
.code { font-size: 28px; font-weight: 800; letter-spacing: 0.12em; color: var(--blue); }
.copy-mini {
  position: relative; height: 32px; padding: 0 12px;
  border: 1px solid var(--border); border-radius: 999px;
  background: var(--surface); color: var(--text-2);
  font-size: 12px; font-weight: 600; cursor: pointer;
  transition: color 0.2s, border-color 0.2s, background 0.2s;
}
.copy-mini::after { content: ''; position: absolute; inset: -7px; }
.copy-mini:hover { background: var(--surface-strong); border-color: var(--border-hover); color: var(--text); }
.copy-mini.done { color: var(--ok); border-color: rgba(12, 166, 120, 0.45); background: rgba(12, 166, 120, 0.08); }
.link-row {
  display: flex; align-items: center; gap: 8px;
  background: #edf2f9; border-radius: 12px; padding: 11px 12px;
  font-size: 12.5px; color: var(--text-2); margin-bottom: 16px;
  white-space: nowrap; overflow: hidden;
}
.link-row span { overflow: hidden; text-overflow: ellipsis; }
.sheet .go {
  width: 100%; height: 46px; border: 0; border-radius: 999px;
  background: var(--blue); color: #fff; font-size: 15px; font-weight: 700; cursor: pointer;
  transition: background 0.25s, color 0.25s;
}
.sheet .go.done { background: rgba(12, 166, 120, 0.1); color: var(--ok); }

/* ---------- 聊天抽屉 ---------- */
.chat-panel {
  position: fixed; top: 0; right: 0; bottom: 0; z-index: 45;
  width: min(340px, 100vw);
  background: var(--surface); border-left: 1px solid var(--border);
  box-shadow: -14px 0 44px rgba(18, 32, 60, 0.12);
  display: flex; flex-direction: column;
  transform: translateX(105%);
  transition: transform 0.32s var(--ease-out);
}
.chat-panel.open { transform: none; }
.chat-head {
  display: flex; align-items: center; gap: 6px;
  padding: 14px 16px; border-bottom: 1px solid rgba(23, 50, 93, 0.06);
  font-weight: 750; font-size: 15px;
}
.chat-head small { color: var(--text-3); font-weight: 500; }
.chat-x { margin-left: auto; border: 0; background: none; font-size: 17px; color: var(--text-3); cursor: pointer; }
.chat-body { flex: 1; overflow-y: auto; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
.msg { max-width: 80%; display: flex; gap: 8px; align-items: flex-start; }
.msg .mini-av {
  width: 26px; height: 26px; border-radius: 50%; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  color: #fff; font-size: 11px; font-weight: 750;
  background: linear-gradient(135deg, #3f8cff, #1f6bff);
}
.msg .mini-av.alt { background: linear-gradient(135deg, #12b886, #0ca678); }
.msg .bubble { background: #edf2f9; border-radius: 4px 14px 14px 14px; padding: 7px 11px; font-size: 13.5px; line-height: 1.5; word-break: break-word; }
.msg .b-name { font-size: 11px; color: var(--text-3); margin-bottom: 2px; }
.msg.mine { align-self: flex-end; flex-direction: row-reverse; }
.msg.mine .bubble { background: var(--blue); color: #fff; border-radius: 14px 4px 14px 14px; }
.msg-sys { text-align: center; font-size: 11px; color: var(--text-3); }
.chat-input { display: flex; gap: 8px; padding: 12px; border-top: 1px solid rgba(23, 50, 93, 0.06); }
.chat-input input {
  flex: 1; min-width: 0; height: 40px; border-radius: 12px;
  border: 1px solid var(--border); background: #edf2f9;
  padding: 0 12px; font-size: 13.5px; color: var(--text); outline: 0;
}
.chat-input input:focus { border-color: rgba(31, 107, 255, 0.45); background: #fff; }
.chat-input button {
  height: 40px; padding: 0 16px; border-radius: 12px; border: 0;
  background: var(--blue); color: #fff; font-weight: 700; font-size: 13px; cursor: pointer;
}
/* ---------- 提示 / 解锁 ---------- */
.toast {
  position: fixed; left: 50%; bottom: 92px; transform: translateX(-50%);
  z-index: 50; padding: 10px 16px; border-radius: 12px;
  background: rgba(23, 50, 93, 0.9); color: #fff; font-size: 12.5px;
  box-shadow: 0 10px 30px rgba(18, 32, 60, 0.3);
}
.audio-unlock {
  position: fixed; left: 50%; bottom: 92px; transform: translateX(-50%);
  z-index: 50; padding: 12px 18px; border-radius: 999px;
  font-size: 14px; font-weight: 600; color: #fff;
  background: var(--brand-grad); box-shadow: 0 10px 30px rgba(23, 50, 93, 0.25);
  cursor: pointer;
}

/* ---------- 移动端（参照腾讯会议/Zoom 手机端：单行图标工具栏 + 紧凑瓷片） ---------- */
@media (max-width: 640px) {
  /* 顶栏：房名单行省略，控件不折行 */
  .topbar { gap: 8px; padding: 10px 12px; }
  .room-title { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 15px; }
  .spacer { display: none; }
  .chip { height: 34px; padding: 0 10px; font-size: 12px; flex-shrink: 0; }
  .chip.hide-m { display: none; }
  .invite-top { white-space: nowrap; flex-shrink: 0; height: 34px; padding: 0 12px; }
  .popover { right: 10px; left: 10px; width: auto; }

  /* 舞台：双列小瓷片，头像缩到瓷片 1/3 左右 */
  .stage { grid-template-columns: repeat(2, 1fr); gap: 10px; padding: 6px 12px 110px; }
  .tile { padding: 18px 10px 14px; border-radius: 18px; gap: 7px; }
  .avatar { width: clamp(64px, 22vw, 84px); height: clamp(64px, 22vw, 84px); font-size: 22px; }
  .owner-badge { right: -10px; }
  .t-name { font-size: 13.5px; }

  /* 底栏：单条图标化（弹幕/说话/聊天/离开圆钮 + 红色结束），贴 iOS 安全区 */
  .bottom-bar {
    bottom: calc(10px + env(safe-area-inset-bottom));
    padding: 0 10px;
  }
  .lbl { display: none !important; } /* 所有按钮只留图标 */
  .chip .info-ico { display: none; }
  .invite-top { width: 34px; padding: 0; }
  .dock { padding: 6px 8px; gap: 4px; border-radius: 999px; }
  .tb-item { min-width: 42px; height: 42px; width: 42px; padding: 0; border-radius: 50%; }
  .tb-item.primary.off { color: var(--danger); }
  .tb-divider { height: 20px; margin: 0 2px; }
  .end-btn { width: 42px; height: 42px; padding: 0; margin-left: 2px; justify-content: center; border-radius: 50%; }
  .fab-badge { top: 0; right: 2px; }
  .dm-pop { width: min(240px, calc(100vw - 32px)); }
  .audio-unlock { bottom: calc(78px + env(safe-area-inset-bottom)); }

  .chat-panel {
    top: auto; left: 0; right: 0; height: 62%; width: auto;
    border-radius: 20px 20px 0 0;
    border: 1px solid var(--border); border-bottom: 0;
    transform: translateY(108%);
  }
  .chat-panel.open { transform: none; }
}
</style>
