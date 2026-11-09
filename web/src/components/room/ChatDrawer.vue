<!--
  文字聊天抽屉：PC 右侧滑入 / 手机底部半屏。
  消息列表与输入框自包含；发送事件交给父级（服务器入账 + 署名），
  服务器回流的消息由父级经聊天层写进 messages 列表。
-->
<template>
  <aside class="chat-panel" :class="{ open }">
    <div class="chat-head">
      会议聊天<small>{{ memberCount }} 人</small>
      <button class="chat-x" @click="emit('close')">✕</button>
    </div>

    <div class="chat-body" ref="bodyEl">
      <div v-if="!messages.length" class="msg-sys">还没有消息，说句打个招呼吧</div>
      <div v-for="(m, i) in messages" :key="i" class="msg" :class="{ mine: m.mine }">
        <div class="mini-av" :class="{ alt: !m.mine }">{{ avatarText(m.name) }}</div>
        <div>
          <div class="bubble">
            <div v-if="!m.mine" class="b-name">{{ m.name }}</div>
            {{ m.text }}
          </div>
        </div>
      </div>
    </div>

    <div class="chat-input">
      <input
        v-model="draft"
        type="text"
        placeholder="发消息…"
        maxlength="200"
        @keydown.enter="sendDraft"
      />
      <button @click="sendDraft">发送</button>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { avatarText } from '../../lib/avatar'
import type { ChatMsg } from '../../composables/useRoomChat'

const props = defineProps<{
  open: boolean
  memberCount: number
  messages: ChatMsg[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'send', text: string): void
}>()

const draft = ref('')
const bodyEl = ref<HTMLElement | null>(null)

/** 消息增加或抽屉打开时，滚到底部 */
watch(
  () => [props.messages.length, props.open] as const,
  () => {
    if (props.open) void nextTick(scrollToEnd)
  },
)

function scrollToEnd() {
  const el = bodyEl.value
  if (el) el.scrollTop = el.scrollHeight
}

/** 给父级/聊天层注册的滚动器（useRoomChat 新消息也会调） */
defineExpose({ scrollToEnd })

function sendDraft() {
  const t = draft.value.trim()
  if (!t) return
  draft.value = ''
  emit('send', t)
  void nextTick(scrollToEnd)
}
</script>

<style scoped>
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

/* ---------- 移动端：底部半屏 ---------- */
@media (max-width: 640px) {
  .chat-panel {
    top: auto; left: 0; right: 0; height: 62%; width: auto;
    border-radius: 20px 20px 0 0;
    border: 1px solid var(--border); border-bottom: 0;
    transform: translateY(108%);
  }
  .chat-panel.open { transform: none; }
}
</style>
