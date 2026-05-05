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

const props = withDefaults(defineProps<{
  active: string
  themeCount?: number
  marketplaceCount?: number
  trayMemoryMb?: number
}>(), {
  themeCount: 0,
  marketplaceCount: 0,
  trayMemoryMb: 11.4,
})

const emit = defineEmits<{
  'update:active': [id: string]
  'panic': []
}>()

const workspace = computed<NavEntry[]>(() => [
  { id: 'library', icon: 'Library', label: t('nav.library'), count: props.themeCount },
  { id: 'creator', icon: 'Brush', label: t('nav.creator'), count: null },
  { id: 'marketplace', icon: 'Globe', label: t('nav.marketplace'), count: props.marketplaceCount },
])

const system = computed<NavEntry[]>(() => [
  { id: 'settings', icon: 'Settings', label: t('nav.settings') },
  { id: 'appearance', icon: 'Moon', label: t('nav.appearance') },
])

function navigate(id: string) {
  emit('update:active', id)
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark"><UiIcon name="Logo" :size="18" /></div>
      <div class="brand-name">
        {{ t('app.name') }}
        <small>{{ t('app.edition').toUpperCase() }} · v1.0</small>
      </div>
    </div>

    <div class="nav-section">
      <h6>{{ t('nav.workspace') }}</h6>
      <button
        v-for="it in workspace"
        :key="it.id"
        :class="['nav-item', { active: active === it.id }]"
        @click="navigate(it.id)"
      >
        <UiIcon :name="it.icon" />
        <span>{{ it.label }}</span>
        <span v-if="it.count !== null && it.count !== undefined" class="nav-count">{{ it.count }}</span>
      </button>
    </div>

    <div class="nav-section">
      <h6>{{ t('nav.system') }}</h6>
      <button
        v-for="it in system"
        :key="it.id"
        :class="['nav-item', { active: active === it.id }]"
        @click="navigate(it.id)"
      >
        <UiIcon :name="it.icon" />
        <span>{{ it.label }}</span>
      </button>
    </div>

    <div class="sidebar-foot">
      <button class="panic" title="Ctrl+Alt+Shift+R" @click="emit('panic')">
        <UiIcon name="Alert" :size="14" />
        <span>パニックリセット</span>
        <span class="kbd">⌃⌥⇧R</span>
      </button>
      <div class="session">
        <span class="dot" />
        <span>トレイ常駐 · {{ trayMemoryMb.toFixed(1) }} MB</span>
      </div>
    </div>
  </aside>
</template>
