<!--
  会议现场顶栏：房名 + 人数 chip（含计时）+ 邀请按钮。
  纯展示组件：浮层开关、弹层开合都由父级响应事件后驱动。
-->
<template>
  <header class="topbar">
    <BrandLogo :size="26" />
    <div class="room-title">{{ roomName }}</div>
    <div class="spacer"></div>
    <button class="chip" @click.stop="emit('toggle-members')">
      <svg viewBox="0 0 24 24" width="15" height="15"><circle cx="9" cy="8" r="3.2" fill="currentColor"/><path d="M3.5 19c.6-3.2 2.8-5 5.5-5s4.9 1.8 5.5 5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/><circle cx="16.5" cy="9" r="2.6" fill="currentColor" opacity=".55"/><path d="M16.5 14.5c2.3.3 3.8 1.9 4.3 4.5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" opacity=".55"/></svg>
      <span class="lbl"><b>{{ memberCount }}</b> 人</span>
      <svg class="info-ico" viewBox="0 0 16 16"><circle cx="8" cy="8" r="6.8" fill="none" stroke="currentColor" stroke-width="1.4"/><circle cx="8" cy="4.9" r="1" fill="currentColor"/><rect x="7.3" y="7" width="1.4" height="4.8" rx="0.7" fill="currentColor"/></svg>
    </button>
    <div class="chip hide-m">{{ timerText }}</div>
    <button class="invite-top" @click="emit('invite')">
      <svg viewBox="0 0 24 24" width="15" height="15"><circle cx="10" cy="8" r="3.2" fill="currentColor"/><path d="M4.5 19c.6-3.2 2.8-5 5.5-5s4.9 1.8 5.5 5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/><path d="M18 6v6M15 9h6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
      <span class="lbl">邀请</span>
    </button>
  </header>
</template>

<script setup lang="ts">
import BrandLogo from '../BrandLogo.vue'

defineProps<{
  roomName: string
  memberCount: number
  /** 顶栏计时 hh:mm(:ss) */
  timerText: string
}>()

const emit = defineEmits<{
  (e: 'toggle-members'): void
  (e: 'invite'): void
}>()
</script>

<style scoped>
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

/* ---------- 移动端：房名单行省略，控件图标化不折行 ---------- */
@media (max-width: 640px) {
  .topbar { gap: 8px; padding: 10px 12px; }
  .room-title { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 15px; }
  .spacer { display: none; }
  .chip { height: 34px; padding: 0 10px; font-size: 12px; flex-shrink: 0; }
  .chip.hide-m { display: none; }
  .chip .info-ico { display: none; }
  .invite-top { white-space: nowrap; flex-shrink: 0; height: 34px; padding: 0 12px; }
  .invite-top { width: 34px; padding: 0; }
}
</style>
