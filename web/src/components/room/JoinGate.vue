<!--
  加入闸门：昵称输入 + 加入码校验，ukiyo 期唯一的「报名页」。
  只管表单与校验反馈：昵称记住、房间探测（是否需要码）由父级（RoomPage）负责，
  校验通过即 emit('join', nickname)，父级决定是否放行进房。
-->
<template>
  <section class="join-wrap">
    <header class="join-top">
      <BrandLogo :size="26" />
      <span class="join-room-name">{{ roomName }}</span>
    </header>

    <div class="join hero">
      <span class="room-avatar rise" :style="nameColor(roomName)">{{ roomName.slice(0, 1).toUpperCase() }}</span>
      <h1 class="display rise" style="--d: 1">加入「{{ roomName }}」</h1>
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
          @keydown.enter="tryJoin"
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

      <!-- 加入码闸门：房间使用中且链接未带码时，必须填对码才能进 -->
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
          @keydown.enter="tryJoin"
        />
        <span class="code-mark" :class="{ ok: codeInputOk }">{{ codeInputOk ? '✓' : '' }}</span>
      </div>
      <p v-if="codeErr" class="code-err">{{ codeErr }}</p>

      <button class="cta rise" style="--d: 4" :disabled="!nicknameValid || needCode === null" @click="tryJoin">
        <span>进入房间</span>
        <svg viewBox="0 0 24 24" width="17" height="17" fill="none">
          <path d="M5 12h14M13 6l6 6-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>

      <p class="hint text-3 rise" style="--d: 5">每个房间的昵称会记住，下次直接进。</p>
      <p v-if="micErr" class="mic-err">{{ micErr }}</p>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import BrandLogo from '../BrandLogo.vue'
import { nameColor } from '../../lib/avatar'
import { normalizeCode } from '../../lib/invite'
import { randomNickname } from '../../lib/random'

const props = defineProps<{
  roomName: string
  /** 三态：null=房间探测中（禁用进入按钮） false=不用填码 true=要填码 */
  needCode: boolean | null
  /** 链接自带的码是否已经生效（免填） */
  codeVerified: boolean
  /** 房间的正确加入码（校验基准） */
  code: string
  /** 连接失败文案（父级语音会话产出） */
  micErr?: string
}>()

const emit = defineEmits<{ (e: 'join', nickname: string): void }>()

const nickname = ref('')
const nickInput = ref<HTMLInputElement | null>(null)
const nicknameValid = computed(() => nickname.value.trim().length > 0)

const codeInput = ref('')
const codeErr = ref('')
const codeInputOk = computed(() => normalizeCode(codeInput.value) === normalizeCode(props.code))

function rollNickname() {
  nickname.value = randomNickname()
  nickInput.value?.focus()
}

function tryJoin() {
  const nick = nickname.value.trim()
  if (!nicknameValid.value) return
  if (props.needCode === null) return // 房间还在探测，先别动
  if (props.needCode && !props.codeVerified && !codeInputOk.value) {
    codeErr.value = '加入码不对，向房主确认一下'
    return
  }
  emit('join', nick)
}
</script>

<style scoped>
/* ---------- 门禁页 ---------- */
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
</style>
