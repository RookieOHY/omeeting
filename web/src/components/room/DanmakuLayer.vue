<!--
  弹幕飞行层：全屏 fixed，纯展示。条目由父级 useDanmaku 池供给，
  飞完（CSS animation 结束）发回收事件，避免容器无限增长。
-->
<template>
  <div v-if="on" class="danmaku-layer" aria-hidden="true">
    <span
      v-for="d in items"
      :key="d.id"
      class="danmaku-item"
      :style="{ top: d.top + '%' }"
      @animationend="emit('remove', d.id)"
    >{{ d.text }}</span>
  </div>
</template>

<script setup lang="ts">
import type { DanmakuItem } from '../../composables/useDanmaku'

defineProps<{
  /** 显示层总开关（产品定案：默认常开） */
  on: boolean
  items: DanmakuItem[]
}>()

const emit = defineEmits<{ (e: 'remove', id: number): void }>()
</script>

<style scoped>
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
</style>
