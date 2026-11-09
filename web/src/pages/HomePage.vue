<template>
  <div class="page home">
    <!-- 顶部公告条（借鉴 promo bar 的克制版） -->
    <header class="top rise" style="--d: 0">
      <div class="brand">
        <BrandLogo :size="38" />
        <span class="name">O-Meeting</span>
      </div>
    </header>

    <!-- Hero -->
    <section class="hero">
      <p class="kicker rise" style="--d: 1">
        <span>✦</span> 免注册 · 浏览器直达 · 双端适配
      </p>

      <h1 class="display rise" style="--d: 2">
        开一间语音房，<br />
        <span class="grad-text">一条链接拉所有人进来。</span>
      </h1>

      <p class="lead rise" style="--d: 3">
        一条链接，点开就聊。<span class="text-3">不装 App、不用注册，手机电脑随开随用。</span>
      </p>

      <!-- 房间输入器 -->
      <div class="composer rise" style="--d: 4" :class="{ focused, filled: !!name && valid }">
        <span class="hash">#</span>
        <input
          ref="inputEl"
          v-model="name"
          class="composer-input"
          type="text"
          placeholder="给你的房间起个名字，比如「今晚开黑」"
          autocomplete="off"
          spellcheck="false"
          enterkeyhint="go"
          :maxlength="80"
          @focus="focused = true"
          @blur="focused = false"
          @keydown.enter="go"
        />
        <button class="dice" title="随便来一个名字" @click="roll">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none">
            <rect x="3.2" y="3.2" width="17.6" height="17.6" rx="4.6" stroke="currentColor" stroke-width="1.7" />
            <circle cx="8.6" cy="8.6" r="1.5" fill="currentColor" />
            <circle cx="15.4" cy="15.4" r="1.5" fill="currentColor" />
            <circle cx="15.4" cy="8.6" r="1.5" fill="currentColor" />
          </svg>
        </button>
        <button class="go" :disabled="!valid" @click="go">
          <span>进入房间</span>
          <svg viewBox="0 0 24 24" width="17" height="17" fill="none">
            <path d="M5 12h14M13 6l6 6-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      </div>

      <div class="waves rise" style="--d: 4.5" :class="{ hot: name }" aria-hidden="true">
        <i v-for="n in 48" :key="n" :style="`--i:${n}`" />
      </div>

      <p v-if="name && !valid" class="hint hint-error rise" style="--d: 5">
        房间名不能为空、不能包含空格或「/」，40 字以内
      </p>

      <div class="chips rise" style="--d: 5">
        <span class="chips-label text-3">热门场景</span>
        <button
          v-for="c in presets"
          :key="c.name"
          class="chip"
          @click="pick(c.name)"
        >
          <span class="chip-dot" :style="dotStyle(c.name)" />
          {{ c.name }}
        </button>
      </div>
    </section>

    <!-- 场景跑马灯 -->
    <div class="marquee rise" style="--d: 6" aria-hidden="true">
      <div class="marquee-track">
        <span v-for="(s, i) in [...scenarios, ...scenarios]" :key="i" class="m-item">
          <i class="m-dot" />{{ s }}
        </span>
      </div>
    </div>

    <!-- 最近加入 -->
    <section v-if="recents.length" class="recents rise" style="--d: 8">
      <div class="recents-head">
        <h2>最近加入</h2>
        <button class="clear" @click="clearAll">清空</button>
      </div>
      <ul class="recent-list">
        <li v-for="r in recents" :key="r.room" class="recent-item">
          <span class="avatar" :style="avatar(r.room)">{{ r.room.slice(0, 1).toUpperCase() }}</span>
          <a class="recent-main" role="button" :title="`进入 ${r.room}`" @click="enter(r.room)">
            <span class="recent-name">#{{ r.room }}</span>
            <span class="recent-meta text-3">{{ r.nickname ? '我的昵称：' + r.nickname + ' · ' : '' }}{{ relative(r.joinedAt) }}</span>
          </a>
          <span class="recent-actions">
            <button class="icon-btn ghost" title="删除记录" @click="remove(r.room)">
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none">
                <path d="M5 5l14 14M19 5L5 19" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
              </svg>
            </button>
            <button class="icon-btn solid" title="进入房间" @click="enter(r.room)">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none">
                <path d="M5 12h14M13 6l6 6-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            </button>
          </span>
        </li>
      </ul>
    </section>

    <footer class="foot rise" style="--d: 9">
      <a class="coffee" href="https://www.buymeacoffee.com" target="_blank" rel="noopener">☕ Buy me a coffee</a>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { loadRooms, removeRoom, saveRoom, type RoomRecord } from '../lib/rooms'
import { randomRoomName } from '../lib/random'
import BrandLogo from '../components/BrandLogo.vue'

const router = useRouter()

const name = ref('')
const focused = ref(false)
const inputEl = ref<HTMLInputElement | null>(null)
const recents = ref<RoomRecord[]>([])

const valid = computed(() => {
  const n = name.value.trim()
  return n.length > 0 && n.length <= 40 && /^[\p{L}\p{N}_-]+$/u.test(n)
})

const presets = [
  { name: '今晚开黑' },
  { name: '晨会速记' },
  { name: '灵感随聊' },
  { name: '读书会' },
]

const scenarios = ['游戏开黑', '远程晨会', '播客对谈', '读书会', '模拟面试', '双语陪练', '灵感速记', '云自习']

function go() {
  const n = name.value.trim()
  if (!valid.value) return
  saveRoom(n)
  router.push(`/${encodeURIComponent(n)}`)
}

function pick(text: string) {
  name.value = text
  inputEl.value?.focus()
}

function roll() {
  name.value = randomRoomName()
  inputEl.value?.focus()
}

function enter(room: string) {
  router.push(`/${encodeURIComponent(room)}`)
}

function remove(room: string) {
  removeRoom(room)
  recents.value = loadRooms()
}

function clearAll() {
  for (const r of recents.value) removeRoom(r.room)
  recents.value = []
}

function relative(ts: number): string {
  const diff = Date.now() - ts
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  const d = new Date(ts)
  const today = new Date()
  if (d.toDateString() === today.toDateString()) {
    return `今天 ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
  }
  if (diff < 86_400_000 * 7) return `${Math.floor(diff / 86_400_000)} 天前`
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

function avatar(room: string): string {
  let h = 0
  for (const ch of room) h = (h * 31 + ch.codePointAt(0)!) % 360
  return `background: linear-gradient(135deg, hsl(${h} 72% 58%), hsl(${h + 70} 70% 62%))`
}

/** chip 彩点：与房间头像同源的哈希色 */
function dotStyle(name: string): string {
  let h = 0
  for (const ch of name) h = (h * 31 + ch.codePointAt(0)!) % 360
  return `background: hsl(${h} 72% 58%); box-shadow: 0 0 8px hsl(${h + 70} 70% 62%)`
}

onMounted(() => {
  recents.value = loadRooms()
})
</script>

<style scoped>
/* ---------- 顶栏 ---------- */
.top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

/* ---------- Hero ---------- */
.hero {
  margin-top: clamp(40px, 10vw, 92px);
  display: flex;
  flex-direction: column;
}

.kicker {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  color: var(--text-2);
  font-size: 13px;
  font-weight: 600;
}

.kicker span {
  color: var(--aqua);
}

.display {
  margin: 18px 0 0;
  font-size: clamp(30px, 7vw, 60px);
  line-height: 1.1;
  font-weight: 800;
  letter-spacing: -0.04em;
  text-wrap: balance;
}

.lead {
  margin: 22px 0 0;
  max-width: 34em;
  font-size: clamp(14.5px, 2.4vw, 16.5px);
  line-height: 1.75;
}

/* ---------- 输入器 ---------- */
.composer {
  position: relative;
  margin-top: clamp(26px, 6vw, 42px);
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 7px 7px 22px;
  border-radius: calc(infinity * 1px);
  background: var(--surface);
  border: 1px solid var(--border);
  transition: border-color 0.25s var(--ease-out), background 0.25s;
  z-index: 0;
}

.composer::before {
  content: '';
  position: absolute;
  inset: -1px;
  border-radius: inherit;
  background: linear-gradient(100deg, rgba(31, 107, 255, 0.45), rgba(111, 164, 255, 0.45));
  filter: blur(14px);
  opacity: 0;
  transition: opacity 0.4s var(--ease-out);
  z-index: -1;
}

.composer.focused {
  border-color: transparent;
  background: var(--surface-strong);
}

.composer.focused::before { opacity: 0.7; }
.composer.filled::before { opacity: 0.35; }

.hash {
  color: var(--text-3);
  font-weight: 700;
  font-size: 17px;
  user-select: none;
}

.composer-input {
  flex: 1;
  min-width: 0;
  height: 46px;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--text);
  font-size: 16px;
  font-weight: 500;
}

.composer-input::placeholder {
  color: var(--text-3);
}

.dice,
.go {
  flex-shrink: 0;
  border: 0;
  border-radius: 999px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
}

.dice {
  width: 42px;
  height: 42px;
  background: transparent;
  color: var(--text-3);
  transition: transform 0.35s var(--ease-spring), color 0.2s, background 0.2s;
}

.dice:hover {
  color: var(--text);
  background: var(--surface-strong);
  transform: rotate(90deg) scale(1.06);
}

.go {
  height: 46px;
  padding: 0 20px 0 24px;
  background: var(--brand-grad);
  color: #fff;
  font-size: 14.5px;
  font-weight: 750;
  position: relative;
  overflow: hidden;
  transition: filter 0.2s, transform 0.15s var(--ease-out), opacity 0.2s;
}

.go svg { transition: transform 0.25s var(--ease-spring); }
.go:hover:not(:disabled) { filter: brightness(1.1); }
.go:hover:not(:disabled) svg { transform: translateX(3px); }
.go:active:not(:disabled) { transform: scale(0.96); }
.go:disabled { opacity: 0.35; cursor: not-allowed; }

.go::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(115deg, transparent 30%, rgba(255, 255, 255, 0.5) 50%, transparent 70%);
  transform: translateX(-110%);
}

.go:hover:not(:disabled)::after {
  animation: sheen 0.9s var(--ease-out);
}

@keyframes sheen {
  to { transform: translateX(110%); }
}

/* ---------- 波形 ---------- */
.waves {
  display: flex;
  align-items: end;
  justify-content: space-between; /* 条数固定，自动铺满整行，双端都不断流 */
  gap: 3px;
  width: 100%;
  height: 44px;
  margin: 16px 0 0;
  opacity: 0.5;
  overflow: hidden;
  transition: opacity 0.3s;
  -webkit-mask-image: linear-gradient(90deg, transparent, #000 6%, #000 94%, transparent);
  mask-image: linear-gradient(90deg, transparent, #000 6%, #000 94%, transparent);
}

.waves i {
  flex: none;
  width: 3px;
  height: 20%;
  border-radius: 3px;
  background: #b9c8de;
  animation: wave 1.6s ease-in-out calc(var(--i) * -0.11s) infinite;
}

.waves.hot { opacity: 1; }

.waves.hot i {
  background: linear-gradient(180deg, #1f6bff, rgba(31, 107, 255, 0.12));
  animation-duration: 0.85s;
}

@keyframes wave {
  0%, 100% { height: 18%; }
  50% { height: 30%; }
}

.hint {
  margin: 12px 4px 0;
  font-size: 13px;
  color: var(--text-2);
}

.hint-error {
  color: var(--danger);
}

/* ---------- chips ---------- */
.chips {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 22px;
  overflow-x: auto;
  scrollbar-width: none;
  padding-bottom: 2px;
}

.chips::-webkit-scrollbar { display: none; }

.chips-label {
  flex-shrink: 0;
  font-size: 12.5px;
  letter-spacing: 0.05em;
}

.chip {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 34px;
  padding: 0 14px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--surface);
  color: var(--text-2);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s, border-color 0.2s, background 0.2s, transform 0.15s var(--ease-out);
}

.chip:hover {
  color: var(--text);
  border-color: var(--border-hover);
  transform: translateY(-1px);
}

.chip-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

/* ---------- 跑马灯 ---------- */
.marquee {
  margin-top: clamp(36px, 7vw, 56px);
  overflow: hidden;
  -webkit-mask-image: linear-gradient(90deg, transparent, #000 12%, #000 88%, transparent);
  mask-image: linear-gradient(90deg, transparent, #000 12%, #000 88%, transparent);
}

.marquee-track {
  display: flex;
  width: max-content;
  animation: marquee 26s linear infinite;
}

.marquee:hover .marquee-track {
  animation-play-state: paused;
}

.m-item {
  display: inline-flex;
  align-items: center;
  gap: 9px;
  padding: 0 18px;
  color: var(--text-3);
  font-size: 13px;
  font-weight: 650;
  letter-spacing: 0.08em;
  white-space: nowrap;
}

.m-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--blue);
  opacity: 0.7;
}

@keyframes marquee {
  to { transform: translateX(-50%); }
}

/* ---------- 最近加入 ---------- */
.recents {
  margin-top: clamp(44px, 9vw, 72px);
}

.recents-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 6px;
}

.recents-head h2 {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
  color: var(--text-2);
}

.clear {
  border: 0;
  background: transparent;
  color: var(--text-3);
  font-size: 12.5px;
  cursor: pointer;
  transition: color 0.2s;
}

.clear:hover { color: var(--danger); }

.recent-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.recent-item {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 13px 16px 13px 13px;
  border-radius: var(--r-md);
  transition: background 0.2s;
}

.recent-item:hover { background: var(--surface); }

.avatar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 12px;
  color: rgba(255, 255, 255, 0.92);
  font-weight: 750;
  font-size: 15px;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
}

.recent-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  cursor: pointer;
}

.recent-name {
  font-weight: 700;
  font-size: 14.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: color 0.2s;
}

.recent-item:hover .recent-name { color: var(--aqua); }

.recent-meta {
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-actions {
  flex-shrink: 0;
  display: flex;
  gap: 6px;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 38px;
  height: 38px;
  border: 0;
  border-radius: 12px;
  cursor: pointer;
  transition: transform 0.15s var(--ease-out), background 0.2s;
}

.icon-btn:active { transform: scale(0.9); }

.icon-btn.ghost {
  background: transparent;
  color: var(--text-3);
}

.icon-btn.ghost:hover {
  background: var(--surface-strong);
  color: var(--text);
}

.icon-btn.solid {
  background: var(--brand-grad);
  color: #fff;
}

.icon-btn.solid:hover { filter: brightness(1.12); }

.foot {
  margin-top: clamp(52px, 10vw, 84px);
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 10px;
  font-size: 11.5px;
  letter-spacing: 0.04em;
}

.sep { opacity: 0.5; }

.coffee {
  color: var(--text-3);
  text-decoration: none;
  transition: color 0.2s;
}
.coffee:hover { color: var(--blue); }

/* ---------- 移动端 ---------- */
@media (max-width: 620px) {
  .go {
    padding: 0 16px 0 20px;
  }
}

.rise {
  opacity: 0;
  animation: rise 0.7s var(--ease-out) forwards;
  animation-delay: calc(var(--d, 0) * 0.09s);
}

@keyframes rise {
  from { opacity: 0; transform: translateY(14px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
