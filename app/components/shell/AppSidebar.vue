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
