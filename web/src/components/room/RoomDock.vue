<!--
  底部工具栏：腾讯会议式单条（图标+小字，右侧红色结束钮），浮一条弹幕输入。
  纯交互转发：语音状态由会话层产出，按钮只负责发事件；弹幕输入文字用
  defineModel 与父级的 useDanmaku 双向绑定。
-->
<template>
  <div class="bottom-bar">
    <div class="dock">
      <!-- 弹幕：关=开输入条；开=收起输入条（显示层不受此影响，默认常开） -->
      <button v-if="!dmOpen" class="tb-item" @click="emit('toggle-dm')">
        <svg viewBox="0 0 24 24" width="18" height="18"><rect x="3.2" y="5" width="17.6" height="13" rx="4.2" stroke="currentColor" stroke-width="1.8" fill="none"/><path d="M7.5 10h9M7.5 13.4h5.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>
        <span class="lbl">弹幕</span>
      </button>
      <button v-else class="tb-item" title="收起弹幕输入" @click="emit('toggle-dm')">
        <svg viewBox="0 0 24 24" width="18" height="18"><rect x="3.2" y="5" width="17.6" height="13" rx="4.2" stroke="currentColor" stroke-width="1.8" fill="none"/><path d="M7.5 10h9M7.5 13.4h5.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>
        <span class="lbl">弹幕</span>
      </button>

      <!-- 说话：长按=按住说话，单击=切换持续开麦（判定在会话层的 PTT 状态机里）；
           被房主闭麦 = 禁用，图标加划线 |
      -->
      <button
        class="tb-item primary"
        :class="{ off: micAirborne, locked: ownerMuted }"
        :disabled="micBusy || ownerMuted"
        @pointerdown="emit('talk-down')"
        @pointerup="emit('talk-up')"
        @pointerleave="emit('talk-cancel')"
        @contextmenu.prevent
      >
        <svg viewBox="0 0 24 24" width="18" height="18"><rect x="9" y="3" width="6" height="11" rx="3" fill="currentColor"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/></svg>
        <span class="lbl">{{ micLabel }}</span>
      </button>

      <!-- 文字聊天抽屉（右侧滑出 / 手机底部半屏） -->
      <button class="tb-item" @click="emit('toggle-chat')">
        <svg viewBox="0 0 24 24" width="18" height="18"><path d="M12 4.2c-4.2 0-7.6 2.8-7.6 6.3 0 2 1.1 3.8 2.9 5l-.6 3 3.3-1.7c.65.13 1.32.2 2 .2 4.2 0 7.6-2.8 7.6-6.5S16.2 4.2 12 4.2z" stroke="currentColor" stroke-width="1.8" fill="none" stroke-linejoin="round"/></svg>
        <span class="lbl">聊天</span>
        <span v-if="unread > 0" class="fab-badge">{{ unread }}</span>
      </button>

      <span class="tb-divider" aria-hidden="true"></span>

      <button class="tb-item" @click="emit('leave')">
        <svg viewBox="0 0 24 24" width="18" height="18"><path d="M15 4h3.5A1.5 1.5 0 0 1 20 5.5v13a1.5 1.5 0 0 1-1.5 1.5H15M10 8l-4 4 4 4M6 12h10" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
        <span class="lbl">离开房间</span>
      </button>
      <!-- 结束房间：只有房主可见（真解散待服务端支持，先退出自己） -->
      <button v-if="isOwner" class="end-btn" @click="emit('end')">
        <svg viewBox="0 0 24 24" width="15" height="15"><path d="M12 3v8" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/><path d="M6.6 6.6a8 8 0 1 0 10.8 0" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round"/></svg>
        <span class="lbl">结束房间</span>
      </button>
    </div>

    <!-- 弹幕输入条：浮在工具栏上方，不挤占栏内空间 -->
    <div v-if="dmOpen" class="dm-pop">
      <input
        ref="dmInputEl"
        v-model="dmText"
        type="text"
        placeholder="发条弹幕，回车发射"
        maxlength="60"
        @keydown.enter="emit('dm-send')"
        @keydown.esc="emit('toggle-dm')"
      />
    </div>

    <p v-if="micErr" class="dock-err">{{ micErr }}</p>  </div>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'

const props = defineProps<{
  dmOpen: boolean
  /** 「按住说话 / 关麦 / 松开 结束」 —— 会话层 PTT 状态机的展示面 */
  micLabel: string
  micBusy: boolean
  micAirborne: boolean
  /** 我被房主闭麦中：说话按钮禁用（解除后复活但不自动开麦） */
  ownerMuted: boolean
  unread: number
  isOwner: boolean
  /** 连接错误提示（浮在工具栏上方） */
  micErr?: string
}>()

const emit = defineEmits<{
  (e: 'toggle-dm'): void
  (e: 'talk-down'): void
  (e: 'talk-up'): void
  (e: 'talk-cancel'): void
  (e: 'toggle-chat'): void
  (e: 'leave'): void
  (e: 'end'): void
  (e: 'dm-send'): void
}>()

/** 弹幕输入文字：与父级 useDanmaku().dmText 双向绑定 */
const dmText = defineModel<string>('dmText', { default: '' })

/* 开输入条即自动聚焦，敲完直接回车发射 */
const dmInputEl = ref<HTMLInputElement | null>(null)
watch(
  () => props.dmOpen,
  (open) => {
    if (open) void nextTick(() => dmInputEl.value?.focus())
  },
)
</script>

<style scoped>
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
.tb-item:disabled { opacity: 0.5; cursor: not-allowed; }
.tb-item.primary.locked { color: var(--danger); opacity: 0.45; }
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

/* ---------- 移动端：单排圆钮纯图标 + 贴 iOS 安全区 ---------- */
@media (max-width: 640px) {
  .bottom-bar {
    bottom: calc(10px + env(safe-area-inset-bottom));
    padding: 0 10px;
  }
  .lbl { display: none !important; } /* 所有按钮只留图标 */
  .dock { padding: 6px 8px; gap: 4px; border-radius: 999px; }
  .tb-item { min-width: 42px; height: 42px; width: 42px; padding: 0; border-radius: 50%; }
  .tb-item.primary.off { color: var(--danger); }
  .tb-divider { height: 20px; margin: 0 2px; }
  .end-btn { width: 42px; height: 42px; padding: 0; margin-left: 2px; justify-content: center; border-radius: 50%; }
  .fab-badge { top: 0; right: 2px; }
  .dm-pop { width: min(240px, calc(100vw - 32px)); }
}
</style>
