<template>
  <span class="brand-mark" :style="{ width: `${size}px`, height: `${size}px` }" aria-hidden="true">
    <svg viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="om-grad" x1="8" y1="4" x2="40" y2="44" gradientUnits="userSpaceOnUse">
          <stop stop-color="#3AD2B5" />
          <stop offset="0.55" stop-color="#5B8CFF" />
          <stop offset="1" stop-color="#8F6BFF" />
        </linearGradient>
      </defs>
      <!-- 环形声波柱：字母 O × 声纹 -->
      <rect
        v-for="(h, i) in bars"
        :key="i"
        x="22.4"
        :y="12 - h"
        width="3.2"
        :height="h"
        rx="1.6"
        :transform="`rotate(${i * 45} 24 24)`"
        fill="url(#om-grad)"
      />
      <!-- 中心点：会议中的人 -->
      <circle cx="24" cy="24" r="3" fill="url(#om-grad)" opacity="0.85" />
    </svg>
  </span>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ size?: number }>(), { size: 36 })
// 高低起伏模拟说话的波形节奏
const bars = [12, 6, 10, 7, 12, 6, 10, 7]
</script>

<style scoped>
.brand-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.brand-mark svg {
  width: 100%;
  height: 100%;
  filter: drop-shadow(0 4px 14px rgba(31, 107, 255, 0.3));
}
</style>
