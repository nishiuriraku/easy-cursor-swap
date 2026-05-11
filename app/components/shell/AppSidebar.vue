<script setup lang="ts">
/**
 * サイドバー: Workspace / System のナビゲーションとパニックリセット導線。
 * `active` プロップで現在地をハイライト。クリックで `update:active` を emit。
 */
import { computed } from 'vue'
import { useI18n } from '~/composables/useI18n'
// UiIcon は Nuxt の自動インポートで解決される

const { t } = useI18n()

interface NavEntry {
  id: string
  label: string
  icon: string
  count?: number | null
}

const props = withDefaults(
  defineProps<{
    active: string
    themeCount?: number
    marketplaceCount?: number
    trayMemoryMb?: number
  }>(),
  {
    themeCount: 0,
    marketplaceCount: 0,
    trayMemoryMb: 11.4,
  },
)

const emit = defineEmits<{
  'update:active': [id: string]
  panic: []
}>()

const workspace = computed<NavEntry[]>(() => [
  { id: 'library', icon: 'Library', label: t('nav.library'), count: props.themeCount },
  { id: 'creator', icon: 'Brush', label: t('nav.creator'), count: null },
  { id: 'marketplace', icon: 'Globe', label: t('nav.marketplace'), count: props.marketplaceCount },
])

const system = computed<NavEntry[]>(() => [
  { id: 'settings', icon: 'Settings', label: t('nav.settings') },
])

function navigate(id: string) {
  emit('update:active', id)
}
</script>

<template>
  <aside class="sidebar" :aria-label="t('nav.workspace')">
    <div class="brand" aria-hidden="true">
      <div class="brand-mark"><UiIcon name="Logo" :size="18" /></div>
      <div class="brand-name">
        {{ t('app.name') }}
        <small>{{ t('app.edition').toUpperCase() }} · v1.0</small>
      </div>
    </div>

    <nav :aria-label="t('nav.workspace')">
      <h6 aria-hidden="true">{{ t('nav.workspace') }}</h6>
      <button
        v-for="it in workspace"
        :key="it.id"
        :class="['nav-item', { active: active === it.id }]"
        :aria-current="active === it.id ? 'page' : undefined"
        @click="navigate(it.id)"
      >
        <UiIcon :name="it.icon" aria-hidden="true" />
        <span>{{ it.label }}</span>
        <span
          v-if="it.count !== null && it.count !== undefined"
          class="nav-count"
          :aria-label="`${it.count}件`"
          >{{ it.count }}</span
        >
      </button>
    </nav>

    <nav :aria-label="t('nav.system')">
      <h6 aria-hidden="true">{{ t('nav.system') }}</h6>
      <button
        v-for="it in system"
        :key="it.id"
        :class="['nav-item', { active: active === it.id }]"
        :aria-current="active === it.id ? 'page' : undefined"
        @click="navigate(it.id)"
      >
        <UiIcon :name="it.icon" aria-hidden="true" />
        <span>{{ it.label }}</span>
      </button>
    </nav>

    <div class="sidebar-foot">
      <button
        class="panic"
        :aria-label="t('common.panic') + ' (Ctrl+Alt+Shift+R)'"
        title="Ctrl+Alt+Shift+R"
        @click="emit('panic')"
      >
        <UiIcon name="Alert" :size="14" aria-hidden="true" />
        <span>{{ t('common.panic') }}</span>
        <span class="kbd" aria-hidden="true">⌃⌥⇧R</span>
      </button>
      <div class="session" aria-live="polite" aria-atomic="true">
        <span class="dot" aria-hidden="true" />
        <span>{{ t('common.trayResident') }} · {{ trayMemoryMb.toFixed(1) }} MB</span>
      </div>
    </div>
  </aside>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* ─────────────────────────────────────────────────────────────
 * シェル本体
 * ────────────────────────────────────────────────────────────*/
.sidebar {
  @apply flex flex-col gap-[18px] overflow-y-auto border-r border-line px-3 py-3.5;
  background: linear-gradient(180deg, var(--bg-sidebar-from), var(--bg-sidebar-to));
}

/* ─────────────────────────────────────────────────────────────
 * ブランド (ロゴ + 名前)
 * ────────────────────────────────────────────────────────────*/
.brand {
  @apply flex items-center gap-2.5 border-b border-line px-2 pb-3.5 pt-1.5;
}
.brand-mark {
  @apply relative grid size-8 place-items-center rounded-[8px];
  background: linear-gradient(135deg, var(--accent) 0%, #5dd9bd 100%);
  box-shadow:
    0 6px 16px -6px rgba(124, 242, 212, 0.55),
    inset 0 1px 0 rgba(255, 255, 255, 0.4);
  color: #0a0b0f; /* dark mode: 黒い Logo を accent グラデ上に */
}
.brand-mark::after {
  content: '';
  position: absolute;
  inset: 1px;
  border-radius: 7px;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.18), transparent 40%);
  pointer-events: none;
}
:where(html.light) .brand-mark {
  background: linear-gradient(135deg, var(--accent), #0c8a6c);
  color: #fff; /* light mode: 白い Logo を accent (#0fa885) グラデ上に */
}
.brand-name {
  @apply flex flex-col gap-[3px] font-display text-[15px] font-semibold leading-[1.1] tracking-[-0.01em];
}
.brand-name small {
  @apply font-mono text-[9.5px] tracking-[0.08em] text-fg-mute;
}

/* ─────────────────────────────────────────────────────────────
 * ナビゲーション
 *
 * NOTE: nav 要素には class を当てず、h6 もブラウザデフォルトで表示する。
 * 元の global.css には `.nav-section h6` 用の uppercase / font-mono ルールが
 * あったが、template が `.nav-section` クラスを付けていなかったため dead code。
 * baseline の見た目 (h6 デフォルト) と一致させるため意図的に未スタイル。
 * ────────────────────────────────────────────────────────────*/
.nav-item {
  @apply relative flex w-full cursor-pointer items-center gap-2.5 rounded-[7px] border border-transparent bg-transparent px-2.5 py-[7px] text-left text-[13px] font-medium text-fg-dim;
  transition:
    background 0.12s,
    color 0.12s;
}
.nav-item > svg {
  @apply size-3.5 shrink-0 opacity-85;
}
.nav-item.active > svg {
  @apply opacity-100 text-accent;
}
.nav-item:hover {
  background: rgba(255, 255, 255, 0.03);
  color: var(--fg);
}
.nav-item.active {
  background: rgba(255, 255, 255, 0.04);
  color: var(--fg);
  border-color: var(--line-hi);
  box-shadow: var(--shadow-1);
}
:where(html.light) .nav-item.active {
  background: rgba(15, 20, 35, 0.035);
}
.nav-item.active::before {
  content: '';
  position: absolute;
  left: -12px;
  top: 8px;
  bottom: 8px;
  width: 2px;
  background: var(--accent);
  border-radius: 2px;
  box-shadow: 0 0 12px var(--accent);
}
:where(html.light) .nav-item.active::before {
  box-shadow: 0 0 8px rgba(15, 168, 133, 0.5);
}
.nav-count {
  @apply ml-auto rounded-[4px] border border-line bg-white/[0.04] px-1.5 py-px font-mono text-[10.5px] text-fg-mute;
}
.nav-item.active .nav-count {
  @apply border-accent-line bg-accent-dim text-accent;
}

/* ─────────────────────────────────────────────────────────────
 * フッター (パニック + セッション)
 * ────────────────────────────────────────────────────────────*/
.sidebar-foot {
  @apply mt-auto flex flex-col gap-2;
}
.panic {
  @apply flex cursor-pointer items-center gap-2 rounded-[8px] text-[12px] font-medium;
  padding: 9px 12px;
  background: linear-gradient(180deg, rgba(255, 107, 138, 0.1), rgba(255, 107, 138, 0.04));
  border: 1px solid rgba(255, 107, 138, 0.25);
  color: #ffb8c5;
  transition: all 0.15s;
}
.panic:hover {
  border-color: rgba(255, 107, 138, 0.45);
  color: #fff;
  background: linear-gradient(180deg, rgba(255, 107, 138, 0.18), rgba(255, 107, 138, 0.08));
}
.panic .kbd {
  @apply ml-auto font-mono text-[9.5px] tracking-[0.08em];
  color: rgba(255, 255, 255, 0.5);
}
.session {
  @apply flex items-center gap-2.5 border-t border-line px-2 py-2.5 text-[11.5px] text-fg-dim;
}
.session .dot {
  @apply size-1.5 rounded-full bg-accent;
  box-shadow: 0 0 8px var(--accent);
  animation: pulse 2.4s infinite;
}
:where(html.light) .session .dot {
  box-shadow: 0 0 8px rgba(15, 168, 133, 0.5);
}
</style>
