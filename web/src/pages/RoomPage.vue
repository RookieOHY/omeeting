<!--
  房间页：把「加入闸门、语音会话、聊天、弹幕、成员浮层」拼成一场会议。

  本文件只做编排，不实现功能：
  * 语音/花名册/PTT/RTT  → composables/useVoiceSession
  * 聊天列表/未读/发送    → composables/useRoomChat
  * 弹幕飞行池           → composables/useDanmaku
  * 界面块               → components/room/*（各组件自带样式）
-->
<template>
  <div class="room-shell">
    <!-- ============ 加入前：昵称 + 加入码闸门 ============ -->
    <JoinGate
      v-if="!joined"
      :room-name="name"
      :need-code="needCode"
      :code-verified="codeVerified"
      :code="code"
      :mic-err="session.micErr.value"
      @join="onJoinGate"
    />

    <!-- ============ 加入后：会议现场 ============ -->
    <template v-else>
      <RoomTopbar
        :room-name="name"
        :member-count="session.memberCount.value"
        :timer-text="timerText"
        @toggle-members="membersOpen = !membersOpen"
        @invite="inviteOpen = true"
      />

      <!-- 舞台：我 + 远端成员（同一种瓷片） -->
      <main class="stage">
        <MemberTile
          :name="nickname"
          :level="session.levelMe.value"
          :speaking="session.micAirborne.value"
          :owner="session.amIOwner.value"
        />
        <MemberTile
          v-for="r in session.remotes.value"
          :key="r.id"
          :name="session.remoteName(r.id)"
          :level="session.levelRemote[r.id] ?? 0"
          :speaking="(session.levelRemote[r.id] ?? 0) > 0.02"
          :owner="session.remoteIsOwner(r.id)"
        />
      </main>

      <!-- 远端音频（隐藏 <audio>，负责真正出声；默认自动播放被拦时由解锁条兜底） -->
      <audio
        v-for="r in session.remotes.value"
        :key="'a' + r.id"
        :srcObject.prop="r.stream"
        autoplay
        playsinline
        class="remote-audio"
      />

      <!-- 弹幕层（显示默认常开） -->
      <DanmakuLayer
        :on="dm.danmakuOn.value"
        :items="dm.items.value"
        @remove="dm.remove"
      />

      <!-- 底部工具栏 + 弹幕输入条 + 连接错误 -->
      <RoomDock
        :dm-open="dm.dmOpen.value"
        v-model:dm-text="dm.dmText.value"
        :mic-label="session.micLabel.value"
        :mic-busy="session.micBusy.value"
        :mic-airborne="session.micAirborne.value"
        :unread="chat.unread.value"
        :is-owner="session.amIOwner.value"
        :mic-err="session.micErr.value"
        @toggle-dm="dm.dmOpen.value = !dm.dmOpen.value"
        @talk-down="session.pttDown"
        @talk-up="session.pttUp"
        @talk-cancel="session.pttCancel"
        @toggle-chat="chat.toggleChat()"
        @leave="leaveRoom"
        @end="endRoom"
        @dm-send="dm.send()"
      />

      <!-- 成员浮层（人数 chip 下拉；点浮层与 chip 以外区域关闭） -->
      <MembersPopover
        v-if="membersOpen"
        :members="memberRows"
        :is-owner="session.amIOwner.value"
        @mute="muteOne"
        @mute-all="muteAll"
      />

      <!-- 邀请弹层 -->
      <InviteSheet
        v-if="inviteOpen"
        :room-name="name"
        :code="code"
        @close="inviteOpen = false"
      />

      <!-- 聊天抽屉 -->
      <ChatDrawer
        ref="drawerEl"
        :open="chat.chatOpen.value"
        :member-count="session.memberCount.value"
        :messages="chat.messages.value"
        @send="chat.send"
        @close="chat.closeChat()"
      />

      <!-- 全局轻提示 -->
      <div v-if="toast" class="toast">{{ toast }}</div>

      <!-- 声音被浏览器拦截的兜底解锁 -->
      <div v-if="session.audioBlocked.value" class="audio-unlock" @click="unlockManual">🔊 点我开启声音</div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import JoinGate from '../components/room/JoinGate.vue'
import RoomTopbar from '../components/room/RoomTopbar.vue'
import MemberTile from '../components/room/MemberTile.vue'
import RoomDock from '../components/room/RoomDock.vue'
import MembersPopover from '../components/room/MembersPopover.vue'
import InviteSheet from '../components/room/InviteSheet.vue'
import ChatDrawer from '../components/room/ChatDrawer.vue'
import DanmakuLayer from '../components/room/DanmakuLayer.vue'
import type { MemberRow } from '../components/room/types'
import { useVoiceSession } from '../composables/useVoiceSession'
import { useRoomChat } from '../composables/useRoomChat'
import { useDanmaku } from '../composables/useDanmaku'
import { unlockPlayback, ensureRemoteAudio } from '../lib/audio'
import { getNickname, saveRoom } from '../lib/rooms'
import { joinCode, normalizeCode } from '../lib/invite'

/* ---------- 房间身份 ---------- */
const route = useRoute()
const router = useRouter()
/** 房名：URL 路径参数（中文解码），同时是服务器的房间键 */
const name = computed(() => decodeURIComponent(String(route.params.name ?? 'UNNAMED')))
const nickname = ref('')
const joined = ref(false)

/* ---------- 加入码闸门（软门槛：链接带码免填；房间使用中才拦） ---------- */
const code = computed(() => joinCode(name.value))
const needCode = ref<boolean | null>(null)
const urlCode = new URLSearchParams(location.search).get('c') ?? ''
const codeVerified = ref(normalizeCode(urlCode) === normalizeCode(code.value))

/** 闸门提交：记住昵称 → 进房 */
function onJoinGate(nick: string) {
  nickname.value = nick
  saveRoom(name.value, nick)
  enterRoom()
}

/* ---------- 房内时计（顶栏 hh:mm(:ss)） ---------- */
let joinedAt = 0
let clockTimer: number | undefined
const timerText = ref('00:00')

function startRoomClock() {
  if (clockTimer) return
  clockTimer = window.setInterval(() => {
    const s = Math.floor((Date.now() - joinedAt) / 1000)
    const h = Math.floor(s / 3600)
    const m = Math.floor((s % 3600) / 60)
    const two = (n: number) => String(n).padStart(2, '0')
    timerText.value = h > 0 ? `${h}:${two(m)}:${two(s % 60)}` : `${two(m)}:${two(s % 60)}`
  }, 1000)
}

/* ---------- 各功能块装配 ---------- */
/* 轻提示（各处报错/占位功能统一出口） */
const toast = ref('')
let toastTimer: number | undefined
function showToast(msg: string) {
  toast.value = msg
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = window.setTimeout(() => (toast.value = ''), 2200)
}

/* 弹幕：显示默认常开；发送链路 = 聊天层入账成功后再飞本端这一条 */
const dm = useDanmaku((t) => void chat.send(t))

/* 聊天：判「我发的」要等会话层签发通行证 → 用闭包晚绑 */
const chat = useRoomChat({
  nickname,
  getPid: () => session?.getPid(),
  onToast: showToast,
  onOwnDanmaku: (t) => dm.push(t),
})

type VoiceSession = ReturnType<typeof useVoiceSession>
let session: VoiceSession | undefined
session = useVoiceSession({
  room: name,
  nickname,
  getChatSince: () => chat.getSince(),
  onRemoteChat: chat.receive,
})

/* 聊天抽屉：新消息/上屏后由抽屉提供「滚到底」能力 */
const drawerEl = ref<InstanceType<typeof ChatDrawer> | null>(null)
chat.registerScroller(() => drawerEl.value?.scrollToEnd())

/* ---------- 进房 / 离房 ---------- */
function enterRoom() {
  joined.value = true
  joinedAt = Date.now()
  startRoomClock()
  // 用户手势内解锁媒体播放通道（iOS/安卓自动播放拦截）
  unlockPlayback()
  session?.start()
}

function leaveRoom() {
  session?.teardown()
  router.push('/')
}

function endRoom() {
  if (confirm('结束房间后所有成员都将被移出，确定？')) {
    // 前端先行版本：解散语义由服务端上线后补全，当前先退出自己
    leaveRoom()
  }
}

/* ---------- 成员浮层 / 邀请弹层 ---------- */
const membersOpen = ref(false)
const inviteOpen = ref(false)

/** 成员浮层数据：把「我」和远端拉平成统一行（RTT 只有自己的是真值） */
const memberRows = computed<MemberRow[]>(() => {
  const me: MemberRow = {
    id: 'me',
    name: nickname.value || '我',
    owner: session?.amIOwner.value ?? false,
    self: true,
    joinedMs: joinedAt,
    rtt: session?.ownRtt.value ?? null,
    speaking: session?.micAirborne.value ?? false,
  }
  const others: MemberRow[] = (session?.remotes.value ?? []).map((r) => ({
    id: r.id,
    name: session!.remoteName(r.id),
    owner: session!.remoteIsOwner(r.id),
    self: false,
    joinedMs: session!.remoteJoined(r.id),
    rtt: null,
    speaking: (session!.levelRemote[r.id] ?? 0) > 0.02,
  }))
  return [me, ...others]
})

/** 点击 chip 或浮层以外区域才关闭（比 stopPropagation 时序更可靠） */
const closeOnOutside = (e: Event) => {
  if (!(e.target as HTMLElement)?.closest('.chip, .popover')) membersOpen.value = false
}

/* 闭麦占位：真正让全校听见「闭麦」需要服务端停发该路音频，未实现前如实相告 */
function muteOne(m: string) {
  showToast(`「${m}」闭麦需要服务端支持，即将上线`)
}
function muteAll() {
  showToast('一键闭麦需要服务端支持，即将上线')
}

/* ---------- 声音解锁兜底 ---------- */
function unlockManual() {
  unlockPlayback()
  void ensureRemoteAudio().then((blocked) => {
    if (session) session.audioBlocked.value = blocked
  })
}

/* ---------- 生命周期 ---------- */
onMounted(async () => {
  // 本机来过：带出昵称直接进（加入即听）
  const saved = getNickname(name.value)
  if (saved) {
    nickname.value = saved
    enterRoom()
    return
  }
  // 探测房间是否正在使用（有人 → 闸门需要加入码）
  if (codeVerified.value) {
    needCode.value = false
    return
  }
  try {
    const users = await session!.probeRoster()
    needCode.value = users.length > 0
  } catch {
    needCode.value = false // 探测失败宁可放行
  }
})

onBeforeUnmount(() => {
  session?.teardown()
  if (clockTimer) clearInterval(clockTimer)
  if (toastTimer) clearTimeout(toastTimer)
  document.removeEventListener('click', closeOnOutside)
})
</script>

<style scoped>
/* ---------- 壳与舞台 ---------- */
.room-shell { min-height: 100vh; display: flex; flex-direction: column; }

.stage {
  flex: 1; width: min(980px, 100%); margin: 0 auto;
  padding: 8px 18px 150px;
  display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  gap: 14px; align-content: start;
}

.remote-audio { display: none; }

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

/* ---------- 移动端 ---------- */
@media (max-width: 640px) {
  .stage { grid-template-columns: repeat(2, 1fr); gap: 10px; padding: 6px 12px 110px; }
  .audio-unlock { bottom: calc(78px + env(safe-area-inset-bottom)); }
}
</style>
