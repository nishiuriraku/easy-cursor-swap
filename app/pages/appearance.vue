<script setup lang="ts">
/**
 * 外観 / ダークモード連動ペアリング (Phase 5-8)
 *
 * design/settings.jsx の外観セクションを Vue 化したもの。
 * `useAppSettings` / `useThemes` / `ThemePickerModal` と連携して
 * Light/Dark のテーマペアリングを config に永続化する。
 */
import { computed, onMounted, ref, watch } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { invokeTauri } from '~/composables/useTauri'
import { useAppSettings } from '~/composables/useAppSettings'
import { useThemes } from '~/composables/useThemes'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const isDark = ref(true)

const { config: appConfig, load: loadConfig, update: persistConfig } = useAppSettings()
const { themes, refresh: refreshThemes } = useThemes()

// UI ローカル状態 — config が来たら同期する
const detection = ref({
  enabled: true,
  showToast: true,
  perPairShadow: false,
})
const lightThemeId = ref<string | null>(null)
const darkThemeId = ref<string | null>(null)

// テーマ選択モーダル
const pickerOpen = ref<'light' | 'dark' | null>(null)
const pickerSelected = ref<string | null>(null)

const dirty = ref(false)
const saving = ref(false)

const fallbackTheme: ThemeCardData = {
  id: '',
  name: t('appearance.unspecified'),
  author: null,
  version: '—',
  date: '',
  applyCount: 0,
  isFavorite: false,
  isActive: false,
  includedRoles: [],
}

function findTheme(id: string | null): ThemeCardData {
  if (!id) return fallbackTheme
  return (
    themes.value.find((th) => th.id === id) ?? {
      ...fallbackTheme,
      name: t('appearance.uninstalled'),
    }
  )
}

const lightTheme = computed(() => findTheme(lightThemeId.value))
const darkTheme = computed(() => findTheme(darkThemeId.value))

function applyConfigToLocal() {
  const c = appConfig.value
  if (!c) return
  detection.value.enabled = c.dark_mode.enabled
  lightThemeId.value = c.dark_mode.light_theme_id
  darkThemeId.value = c.dark_mode.dark_theme_id
  dirty.value = false
}

async function save() {
  saving.value = true
  try {
    await persistConfig((draft) => {
      draft.dark_mode.enabled = detection.value.enabled
      draft.dark_mode.light_theme_id = lightThemeId.value
      draft.dark_mode.dark_theme_id = darkThemeId.value
    })
    dirty.value = false
  } finally {
    saving.value = false
  }
}

function discard() {
  applyConfigToLocal()
}

function openPicker(side: 'light' | 'dark') {
  pickerSelected.value = side === 'light' ? lightThemeId.value : darkThemeId.value
  pickerOpen.value = side
}

function onPickerUpdate(id: string | null) {
  if (pickerOpen.value === 'light') lightThemeId.value = id
  else if (pickerOpen.value === 'dark') darkThemeId.value = id
}

onMounted(async () => {
  await Promise.all([loadConfig(), refreshThemes()])
  applyConfigToLocal()
  watch(appConfig, applyConfigToLocal)
  try {
    const dark = await invokeTauri<boolean>('get_dark_mode_status')
    if (dark !== null) isDark.value = dark
  } catch {
    // 無視
  }
})

// dirty 検出
watch(
  [detection, lightThemeId, darkThemeId],
  () => {
    if (appConfig.value) dirty.value = true
  },
  { deep: true },
)
</script>

<template>
  <div class="appearance-host">
    <div class="toolbar">
      <div class="bcrumb">
        <span class="crumb">{{ t('settings.breadcrumb') }}</span>
        <span class="sep">/</span>
        <span class="crumb active">{{ t('appearance.breadcrumb') }}</span>
      </div>
      <div />
      <div class="tb-actions">
        <button class="btn ghost" :disabled="!dirty || saving" @click="discard">
          {{ t('common.discard') }}
        </button>
        <button class="btn primary" :disabled="!dirty || saving" @click="save">
          <span v-if="saving" class="spinner" style="width: 13px; height: 13px" />
          <UiIcon v-else name="Check" :size="13" />
          {{ saving ? t('common.saving') : t('common.save') }}
        </button>
      </div>
    </div>

    <div class="content">
      <div class="page-head">
        <div>
          <h1>{{ t('appearance.title') }}</h1>
          <p>{{ t('appearance.description') }}</p>
        </div>
        <div class="right">
          <span class="tag ok">
            <span class="watch-dot" />{{
              detection.enabled ? t('appearance.monitoring') : t('appearance.paused')
            }}
          </span>
        </div>
      </div>

      <!-- OS 状態インスペクター -->
      <div class="prop-section os-state">
        <div class="prop-head">
          Current OS State
          <span class="head-hint">HKCU\…\Personalize\AppsUseLightTheme</span>
        </div>
        <div class="indicator-row">
          <ModeIndicator side="light" :active="!isDark" />
          <ModeIndicator side="dark" :active="isDark" />
        </div>
      </div>

      <h6 class="section-title">Theme Pairing</h6>

      <div class="pairing-grid">
        <PairingSlot
          label="Light Mode"
          sub="AppsUseLightTheme = 1"
          accent="#f5c26b"
          :theme="lightTheme"
          :current="!isDark"
          @change="openPicker('light')"
        />
        <div class="auto-switch">
          <svg
            width="20"
            height="20"
            viewBox="0 0 20 20"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <path d="M3 7h12M11 3l4 4-4 4M17 13H5M9 17l-4-4 4-4" />
          </svg>
          <span>Auto Switch</span>
        </div>
        <PairingSlot
          label="Dark Mode"
          sub="AppsUseLightTheme = 0"
          accent="#7cf2d4"
          :theme="darkTheme"
          :current="isDark"
          @change="openPicker('dark')"
        />
      </div>

      <!-- Detection -->
      <div class="prop-section">
        <div class="prop-head">Detection</div>
        <div class="prop-body">
          <SettingsRow
            :label="t('appearance.toggleEnableLabel')"
            :desc="t('appearance.toggleEnableDesc')"
          >
            <SettingsToggle v-model="detection.enabled" />
          </SettingsRow>
          <SettingsRow
            :label="t('appearance.toggleToastLabel')"
            :desc="t('appearance.toggleToastDesc')"
          >
            <SettingsToggle v-model="detection.showToast" />
          </SettingsRow>
          <SettingsRow
            :label="t('appearance.toggleShadowLabel')"
            :desc="t('appearance.toggleShadowDesc')"
          >
            <SettingsToggle v-model="detection.perPairShadow" />
          </SettingsRow>
        </div>
      </div>
    </div>

    <!-- テーマ選択モーダル -->
    <Transition name="fade">
      <ThemePickerModal
        v-if="pickerOpen"
        :themes="themes"
        :model-value="pickerSelected"
        :title="
          pickerOpen === 'light'
            ? t('appearance.pickerLightTitle')
            : t('appearance.pickerDarkTitle')
        "
        :sub="
          pickerOpen === 'light' ? t('appearance.pickerLightSub') : t('appearance.pickerDarkSub')
        "
        :accent="pickerOpen === 'light' ? '#f5c26b' : '#7cf2d4'"
        @update:model-value="onPickerUpdate"
        @cancel="pickerOpen = null"
      />
    </Transition>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.appearance-host {
  @apply flex h-full flex-col;
}
.content {
  @apply mx-auto w-full max-w-[1100px] flex-1 overflow-y-auto px-7 pb-8 pt-6;
}

.page-head h1 {
  @apply text-[22px];
}

.head-hint {
  @apply font-mono text-[10px] font-normal normal-case tracking-normal text-fg-mute;
}

.os-state {
  @apply mb-5;
}
.indicator-row {
  @apply grid grid-cols-2;
}

.section-title {
  @apply mb-3 mt-0 font-mono text-[10px] font-medium uppercase tracking-[0.16em] text-fg-mute;
}

.pairing-grid {
  @apply mb-6 grid grid-cols-[1fr_auto_1fr] items-stretch gap-4;
}
@media (max-width: 800px) {
  .pairing-grid {
    @apply grid-cols-1;
  }
}

.auto-switch {
  @apply flex flex-col items-center justify-center gap-1.5 font-mono text-[9.5px] uppercase tracking-[0.12em] text-fg-mute;
}

.watch-dot {
  @apply mr-1.5 inline-block size-1.5 rounded-full bg-accent;
  box-shadow: 0 0 6px var(--accent);
}

.prop-body {
  padding: 4px 16px !important;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.18s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
