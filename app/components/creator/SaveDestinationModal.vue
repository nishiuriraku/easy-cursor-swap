<script setup lang="ts">
/**
 * テーマの保存先選択モーダル。
 *
 * Creator の `[Save…]` ボタンから開かれ、3 軸 (保存先 / 上書き or 複製 / 署名) を
 * ユーザーに選ばせる。`?editPath` 由来のとき (= sourceThemeId != null) のみ
 * 「既存テーマの扱い」セクションを表示する。
 *
 * destination='file' で submit したときだけ tauri-plugin-dialog を呼んでパスを取得する。
 * Library 系は invokeTauri に渡すだけなので path 取得は不要。
 */
import { ref, computed, watch } from 'vue'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

interface Props {
  open: boolean
  hasKeystoreSigning: boolean
  /** ?editPath で開かれた場合は元テーマ UUID、新規作成では null */
  sourceThemeId: string | null
  defaultDestination: 'file' | 'library' | 'libraryAndApply'
  metaName: string
}
const props = defineProps<Props>()

export interface SaveSubmitPayload {
  destination: 'file' | 'library' | 'libraryAndApply'
  overwriteExisting: boolean
  sign: boolean
  filePath?: string
  effectiveName: string
}
const emit = defineEmits<{
  (e: 'cancel'): void
  (e: 'submit', payload: SaveSubmitPayload): void
}>()

const destination = ref<Props['defaultDestination']>(props.defaultDestination)
const overwriteExisting = ref(true)
const sign = ref(false)
const nameInput = ref(props.metaName)

watch(
  () => props.open,
  (open) => {
    if (open) {
      destination.value = props.defaultDestination
      overwriteExisting.value = true
      sign.value = false
      nameInput.value = props.metaName
    }
  },
)

const today = computed(() => new Date().toISOString().slice(0, 10))
const namePlaceholder = computed(() =>
  t('saveModal.namePlaceholder').replace('{date}', today.value),
)
const effectiveName = computed(() => nameInput.value.trim() || namePlaceholder.value)
const showOverwriteSection = computed(() => props.sourceThemeId !== null)

async function onSubmit() {
  let filePath: string | undefined
  if (destination.value === 'file') {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const safeName = effectiveName.value.replace(/[^\p{L}\p{N}_-]+/gu, '_').slice(0, 64) || 'theme'
    const picked = await save({
      defaultPath: `${safeName}.cursorpack`,
      filters: [{ name: 'Cursor Pack', extensions: ['cursorpack'] }],
    })
    if (!picked || typeof picked !== 'string') return
    filePath = picked
  }
  emit('submit', {
    destination: destination.value,
    overwriteExisting: showOverwriteSection.value ? overwriteExisting.value : false,
    sign: sign.value,
    filePath,
    effectiveName: effectiveName.value,
  })
}
</script>

<template>
  <div v-if="open" class="sd-overlay" @click.self="emit('cancel')">
    <div class="sd-modal" role="dialog" aria-modal="true">
      <header class="sd-head">
        <h3>{{ t('saveModal.title') }}</h3>
        <button class="btn ghost" :aria-label="t('common.close')" @click="emit('cancel')">✕</button>
      </header>

      <div class="sd-body">
        <fieldset class="sv-fieldset">
          <legend>{{ t('saveModal.destinationLabel') }}</legend>
          <div class="ctl-stack">
            <label class="ctl-row">
              <input v-model="destination" type="radio" name="save-destination" value="file" />
              <span class="ctl-radio" :class="{ on: destination === 'file' }" aria-hidden="true" />
              <span>
                <div class="ctl-label">{{ t('saveModal.destinationFile') }}</div>
                <div class="ctl-sub">{{ t('saveModal.destinationFileSub') }}</div>
              </span>
              <span class="ctl-tail">{{ t('saveModal.destinationFileTail') }}</span>
            </label>
            <label class="ctl-row">
              <input v-model="destination" type="radio" name="save-destination" value="library" />
              <span
                class="ctl-radio"
                :class="{ on: destination === 'library' }"
                aria-hidden="true"
              />
              <span>
                <div class="ctl-label">{{ t('saveModal.destinationLibrary') }}</div>
                <div class="ctl-sub">{{ t('saveModal.destinationLibrarySub') }}</div>
              </span>
              <span class="ctl-tail">{{ t('saveModal.destinationLibraryTail') }}</span>
            </label>
            <label class="ctl-row">
              <input
                v-model="destination"
                type="radio"
                name="save-destination"
                value="libraryAndApply"
              />
              <span
                class="ctl-radio"
                :class="{ on: destination === 'libraryAndApply' }"
                aria-hidden="true"
              />
              <span>
                <div class="ctl-label">{{ t('saveModal.destinationLibraryAndApply') }}</div>
                <div class="ctl-sub">{{ t('saveModal.destinationLibraryAndApplySub') }}</div>
              </span>
              <span class="ctl-tail">{{ t('saveModal.destinationLibraryAndApplyTail') }}</span>
            </label>
          </div>
        </fieldset>

        <fieldset v-if="showOverwriteSection" data-test="overwrite-section" class="sv-fieldset">
          <legend>{{ t('saveModal.overwriteLabel') }}</legend>
          <div class="ctl-stack">
            <label class="ctl-row">
              <input
                v-model="overwriteExisting"
                type="radio"
                name="save-overwrite-existing"
                :value="true"
              />
              <span
                class="ctl-radio"
                :class="{ on: overwriteExisting === true }"
                aria-hidden="true"
              />
              <span>
                <div class="ctl-label">{{ t('saveModal.overwriteOverwrite') }}</div>
                <div class="ctl-sub">{{ t('saveModal.overwriteOverwriteSub') }}</div>
              </span>
            </label>
            <label class="ctl-row">
              <input
                v-model="overwriteExisting"
                type="radio"
                name="save-overwrite-existing"
                :value="false"
              />
              <span
                class="ctl-radio"
                :class="{ on: overwriteExisting === false }"
                aria-hidden="true"
              />
              <span>
                <div class="ctl-label">{{ t('saveModal.overwriteDuplicate') }}</div>
                <div class="ctl-sub">{{ t('saveModal.overwriteDuplicateSub') }}</div>
              </span>
            </label>
          </div>
        </fieldset>

        <div class="sv-divider" />

        <!-- 署名: Variant A の .ctl-check + tick svg を採用。
             SettingsToggle と挙動を揃えるため input[type=checkbox] + :disabled 制御。 -->
        <label class="ctl-row" :class="{ disabled: !hasKeystoreSigning }">
          <input
            v-model="sign"
            type="checkbox"
            data-test="sign-checkbox"
            :disabled="!hasKeystoreSigning"
          />
          <span class="ctl-check" :class="{ on: sign }" aria-hidden="true">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </span>
          <span>
            <div class="ctl-label">{{ t('saveModal.sign') }}</div>
            <div class="ctl-sub">
              {{ hasKeystoreSigning ? t('saveModal.signSub') : t('saveModal.signDisabled') }}
            </div>
          </span>
        </label>

        <section v-if="!props.metaName.trim()" data-test="name-field" class="prop-section">
          <label class="sd-field">
            <span class="sd-field-label">{{ t('saveModal.nameLabel') }}</span>
            <input class="input" v-model="nameInput" type="text" :placeholder="namePlaceholder" />
          </label>
        </section>
      </div>

      <footer class="sd-foot">
        <button class="btn ghost" data-test="cancel-btn" @click="emit('cancel')">
          {{ t('saveModal.cancel') }}
        </button>
        <button class="btn primary" data-test="submit-btn" @click="onSubmit">
          {{ t('saveModal.save') }}
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.sd-overlay {
  @apply fixed inset-0 z-[100] flex items-center justify-center bg-[rgba(10,11,15,0.7)];
}
.sd-modal {
  @apply flex max-h-[90vh] w-[min(520px,92vw)] flex-col rounded-[12px] border border-line;
  background: var(--bg-1, #14161c);
}
.sd-head,
.sd-foot {
  @apply flex items-center justify-between border-b border-line px-4 py-3;
}
.sd-foot {
  @apply justify-end gap-2 border-b-0 border-t border-line;
}
.sd-body {
  @apply flex-1 space-y-4 overflow-y-auto px-4 py-4;
}
.sd-hint {
  @apply text-[11px] text-fg-mute;
}

/* fieldset + legend は Claude Design "Save Dialog Controls" Variant A の
 * legend スタイルを移植 (styles-controls.css :: .sv-fieldset > legend)。 */
.sv-fieldset {
  @apply m-0 border-0 p-0;
}
.sv-fieldset > legend {
  @apply mb-2.5 block font-mono text-[9.5px] font-medium uppercase tracking-[0.14em] text-fg-mute;
}
.ctl-stack {
  @apply flex flex-col gap-0.5;
}
.sv-divider {
  @apply h-px bg-line;
}
.ctl-row.disabled {
  @apply pointer-events-none opacity-50;
}

/* --- Name 入力フィールド --- */
.sd-field {
  @apply flex flex-col gap-1.5;
}
.sd-field-label {
  @apply text-[12px] text-fg-dim;
}
</style>
