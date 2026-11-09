<!--
  舞台瓷片：一个参会人 = 头像（渐变+呼吸圈+房主徽章）+ 昵称 + 麦状态图标。
  自己和远端成员长一模一样，只差数据来源 —— 父级把两者归一后循环渲染。
  level 常态为 0（静音/无声）；speaking 决定呼吸圈和状态图标是否点亮。
-->
<template>
  <article class="tile" :class="{ speaking }">
    <div class="avatar" :style="[nameColor(name), ringStyle(level, speaking)]">
      {{ avatarText(name) }}
      <span v-if="owner" class="owner-badge">房主</span>
    </div>
    <div class="t-name">{{ name }}</div>
    <div class="t-status" :class="{ live: speaking }" :title="speaking ? '开麦中' : '静音'">
      <svg v-if="speaking" viewBox="0 0 24 24" width="13" height="13"><rect x="9" y="3" width="6" height="11" rx="3" fill="currentColor"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/></svg>
      <svg v-else viewBox="0 0 24 24" width="13" height="13"><rect x="9" y="3" width="6" height="11" rx="3" fill="none" stroke="currentColor" stroke-width="1.8"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/><path d="M4 4l16 16" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>
    </div>
  </article>
</template>

<script setup lang="ts">
import { avatarText, nameColor, ringStyle } from '../../lib/avatar'

defineProps<{
  name: string
  /** 实时电平 0..1（呼吸圈强度） */
  level: number
  /** 正在说话/开麦 */
  speaking: boolean
  /** 服务器标记的房主 */
  owner: boolean
}>()
</script>

<style scoped>
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

/* ---------- 移动端：双列小瓷片，头像缩到瓷片 1/3 左右 ---------- */
@media (max-width: 640px) {
  .tile { padding: 18px 10px 14px; border-radius: 18px; gap: 7px; }
  .avatar { width: clamp(64px, 22vw, 84px); height: clamp(64px, 22vw, 84px); font-size: 22px; }
  .owner-badge { right: -10px; }
  .t-name { font-size: 13.5px; }
}
</style>
