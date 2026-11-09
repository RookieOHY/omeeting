<!--
  成员浮层：右上角人数 chip 下拉出来的名单（自己置顶 + 远端成员）。
  展示 RTT/开麦状态/加入时间；房主可对单个人「闭麦」（服务端上线前是占位提示）。
-->
<template>
  <div class="popover" @click.stop>
    <div class="p-head">
      <div class="p-title">成员 <small>{{ members.length }} 人在线</small></div>
      <button v-if="isOwner && members.length > 1" class="mute-all" @click="emit('mute-all')">一键闭麦</button>
    </div>

    <!-- 每一行 = 一名成员：头像 / 名字+房主标 / 状态副行 / 右侧 RTT 或闭麦钮 -->
    <div v-for="m in members" :key="m.id" class="m-row">
      <div class="m-avatar" :class="{ alt: !m.self }">{{ avatarText(m.name) }}</div>
      <div class="m-main">
        <div class="m-name"><span v-if="m.owner" class="owner-tag">房主</span>{{ m.name }}</div>
        <div class="m-sub">
          {{ joinedText(m.joinedMs) }} ·
          <span :class="m.speaking ? 'mic-on' : ''">{{ m.speaking ? '开麦中' : '静音' }}</span>
        </div>
      </div>
      <button v-if="isOwner && !m.self" class="mute-btn" @click="emit('mute', m.name)">
        <svg viewBox="0 0 24 24" width="12" height="12"><rect x="9" y="3" width="6" height="11" rx="3" fill="currentColor"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/><path d="M4 4l16 16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
        闭麦
      </button>
      <div v-else class="m-ping">
        <div class="ms"><span class="pdot" :class="pingDot(m.rtt ?? null)"></span>{{ m.rtt ?? '—' }}{{ m.rtt ? 'ms' : '' }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { avatarText, pingDot } from '../../lib/avatar'
import type { MemberRow } from './types'

const props = defineProps<{
  members: MemberRow[]
  /** 是否房主本人（决定闭麦按钮显隐） */
  isOwner: boolean
}>()

const emit = defineEmits<{
  (e: 'mute', name: string): void
  (e: 'mute-all'): void
}>()

/** 加入时间：yyyy-mm-dd hh:mm（无数据 = 刚加入） */
function joinedText(ms?: number): string {
  if (!ms) return '— 加入'
  const d = new Date(ms)
  const two = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())} ${two(d.getHours())}:${two(d.getMinutes())} 加入`
}
</script>

<style scoped>
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
  color: #fff; font-size: 13px; font-weight: 750; letter-spacing: 1px;
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
.owner-tag {
  font-size: 10px; font-weight: 700; color: var(--blue);
  border: 1px solid rgba(31, 107, 255, 0.4); border-radius: 5px; padding: 1px 5px;
}

/* ---------- 移动端 ---------- */
@media (max-width: 640px) {
  .popover { right: 10px; left: 10px; width: auto; }
}
</style>
