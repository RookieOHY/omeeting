<!--
  邀请弹层：加入码 + 带码链接 + 群发邀请语，全部一键复制。
  自包含：分享链接/邀请语在这里拼装，复制反馈自己管理。
-->
<template>
  <div class="sheet-backdrop" @click.self="emit('close')">
    <div class="sheet">
      <button class="x" @click="emit('close')">✕</button>
      <h3>邀请加入「{{ roomName }}」</h3>
      <div class="code-row">
        <span class="code">{{ code }}</span>
        <button class="copy-mini" :class="{ done: codeCopied }" @click="copyCode">{{ codeCopied ? '✓ 已复制' : '复制码' }}</button>
      </div>
      <div class="link-row"><span>{{ displayUrl }}</span></div>
      <button class="go" :class="{ done: textCopied }" @click="copyFull">{{ textCopied ? '✓ 已复制，去粘贴吧' : '复制邀请语，发到群里' }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { shareText } from '../../lib/invite'
import { copyText, flash } from '../../lib/clipboard'

const props = defineProps<{
  roomName: string
  /** 房间加入码（6 位数字） */
  code: string
}>()

const emit = defineEmits<{ (e: 'close'): void }>()

/** 带码分享链接：当前房址 + ?c=码 */
const shareUrl = computed(() => {
  const u = new URL(location.href)
  // 房址里的中文房名保持人类可读（浏览器地址栏自己会转码）
  u.searchParams.set('c', props.code)
  return u.toString()
})
const displayUrl = computed(
  () => `${location.host}${decodeURIComponent(location.pathname)}?c=${props.code}`,
)

const codeCopied = ref(false)
const textCopied = ref(false)

/** 复制码：带一次点亮反馈 */
async function copyCode() {
  if (await copyText(props.code)) flash(codeCopied)
}

/** 复制整段邀请语（房名 + 链接 + 码） */
async function copyFull() {
  if (await copyText(shareText(props.roomName, shareUrl.value, props.code))) flash(textCopied)
}
</script>

<style scoped>
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
.copy-mini::after { content: ''; position: absolute; inset: -7px; } /* 热区扩大，手机好点 */
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
</style>
