<script setup lang="ts">
/**
 * 設定 → 一般 セクション。
 *
 * UI 言語選択 + 通知トグル 2 つ + マウスポインターサイズ (1〜15 スライダー) +
 * ConfigRecoveryPanel (バックアップ復旧)。
 *
 * カーソルサイズは config.json には永続化されない (OS レジストリ
 * HKCU\Control Panel\Cursors\CursorBaseSize が source of truth)。スライダー変更を
 * 即時 IPC で反映するため、`update:cursor-size-slider` を親 (settings.vue) が
 * `@change` 相当のタイミングで受け取り `set_cursor_base_size` IPC を叩く。
 */

const { t } = useI18n()

const language = defineModel<string>('language', { required: true })
const showApplyToast = defineModel<boolean>('showApplyToast', { required: true })
const applyShadowControl = defineModel<boolean>('applyShadowControl', { required: true })

const props = defineProps<{
  cursorSizeSlider: number
  cursorSizeMin: number
  cursorSizeMax: number
  /** スライダー位置に対応する DWORD 値 (px 表示用) */
  cursorSizePx: number
  cursorSizeBusy: boolean
  cursorSizeError: string | null
}>()

const emit = defineEmits<{
  (e: 'config-restored'): void
  /** スライダー値が確定したとき (`@change` 相当)。親が IPC を発火する。 */
  (e: 'update:cursor-size-slider', value: number): void
}>()

// ドラッグ中の視覚フィードバック用にローカル ref を持つ。親側の値が変わったら同期する。
const localSlider = ref(props.cursorSizeSlider)
watch(
  () => props.cursorSizeSlider,
  (v) => {
    localSlider.value = v
  },
)
const localPxPreview = computed(() => {
  // 親の `cursorSizePx` (= sliderToDword(parentSlider)) と同じ式で local 用を計算する。
  // 親の DWORD 値が必要なくドラッグ中のプレビューが出せる。
  const step = 16
  const min = 32
  return min + step * (localSlider.value - props.cursorSizeMin)
})

function onSliderInput(ev: Event) {
  const raw = (ev.target as HTMLInputElement).value
  const n = Number.parseInt(raw, 10)
  if (Number.isFinite(n)) localSlider.value = n
}

// `@input` (ドラッグ中) は連続発火するので IPC 発火を `@change` に絞る。
// `<input type="range">` の値は文字列なので number に正規化。
function onSliderChange(ev: Event) {
  const raw = (ev.target as HTMLInputElement).value
  const n = Number.parseInt(raw, 10)
  if (!Number.isFinite(n)) return
  emit('update:cursor-size-slider', n)
}
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionGeneral') }}</h1>
      <p>{{ t('settings.descGeneral') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">
        {{ t('settings.groupDisplayLanguage') }}
      </div>
      <div class="prop-body">
        <SettingsRow
          anchor="language"
          :label="t('settings.languageLabel')"
          :desc="t('settings.languageDesc')"
        >
          <UiSelect
            v-model="language"
            width="140px"
            :options="[
              { value: 'ja', label: t('settings.languageOptionJa') },
              { value: 'en', label: t('settings.languageOptionEn') },
            ]"
          />
        </SettingsRow>
      </div>
    </div>

    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupCursorSize') }}</div>
      <div class="prop-body">
        <SettingsRow
          anchor="cursorSize"
          :label="t('settings.cursorSizeLabel')"
          :desc="t('settings.cursorSizeDesc')"
        >
          <div class="cursor-size-control">
            <input
              type="range"
              :min="cursorSizeMin"
              :max="cursorSizeMax"
              step="1"
              :value="localSlider"
              :disabled="cursorSizeBusy"
              :aria-label="t('settings.cursorSizeLabel')"
              :aria-valuemin="cursorSizeMin"
              :aria-valuemax="cursorSizeMax"
              :aria-valuenow="localSlider"
              :aria-valuetext="t('settings.cursorSizeReadout', { px: localPxPreview })"
              @input="onSliderInput"
              @change="onSliderChange"
            />
            <span class="cursor-size-readout">
              {{ t('settings.cursorSizeReadout', { px: localPxPreview }) }}
            </span>
          </div>
        </SettingsRow>
        <p v-if="cursorSizeError" class="cursor-size-error" role="alert">
          {{ t('settings.cursorSizeError', { error: cursorSizeError }) }}
        </p>
      </div>
    </div>

    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupNotifications') }}</div>
      <div class="prop-body">
        <SettingsRow
          anchor="showApplyToast"
          :label="t('settings.showApplyToastLabel')"
          :desc="t('settings.showApplyToastDesc')"
        >
          <SettingsToggle v-model="showApplyToast" />
        </SettingsRow>
        <SettingsRow
          anchor="applyShadowControl"
          :label="t('settings.applyShadowControlLabel')"
          :desc="t('settings.applyShadowControlDesc')"
        >
          <SettingsToggle v-model="applyShadowControl" />
        </SettingsRow>
      </div>
    </div>

    <!-- バックアップが存在する場合のみ復旧パネルを表示 -->
    <ConfigRecoveryPanel @restored="$emit('config-restored')" />
  </section>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* NOTE: dead-var pattern. global.css の .prop-section / .prop-head / .prop-body
 * が cascade で効くため、scoped は layout/spacing 差分のみ保持。 */

.section-head {
  @apply mb-4;
}
.section-head h1 {
  @apply mb-1 mt-0 text-[18px] font-bold;
}
.section-head p {
  @apply m-0 text-[13px];
}
.prop-section {
  border-radius: 12px;
  margin-bottom: 12px;
}
.prop-head {
  padding: 10px 16px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
}
.prop-body {
  padding: 4px 16px;
}

.cursor-size-control {
  @apply flex items-center gap-3;
  min-width: 220px;
}
.cursor-size-control input[type='range'] {
  flex: 1;
  min-width: 160px;
  accent-color: var(--accent, #7cf2d4);
}
.cursor-size-control input[type='range']:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.cursor-size-readout {
  @apply font-mono text-[11px] text-fg-dim;
  min-width: 60px;
  text-align: right;
}
.cursor-size-error {
  @apply mt-2 break-all rounded-md text-[11.5px] text-fg-dim;
  padding: 8px 12px;
  background: rgba(255, 100, 100, 0.08);
  border: 1px solid rgba(255, 100, 100, 0.2);
}
</style>
