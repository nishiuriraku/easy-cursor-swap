<script setup lang="ts">
/**
 * Creator のメタデータタブ。
 *
 * テーマ名 (ja/en)、作者、バージョン、影フラグ、説明、エクスポート状況、
 * およびエクスポート中の進捗バーを含む。
 *
 * 純粋なフォーム + 進捗表示なので 6 つの v-model + 数件の read-only props +
 * 2 つの emit (dismiss-export-message, cancel-export) で creator.vue から切り出している。
 */
import { computed } from 'vue'
import { useI18n } from '~/composables/useI18n'
import type { Hotspot } from '~/composables/useCreatorAssets'

const { t } = useI18n()

const metaName = defineModel<string>('metaName', { required: true })
const metaNameEn = defineModel<string>('metaNameEn', { required: true })
const metaAuthor = defineModel<string>('metaAuthor', { required: true })
const metaVersion = defineModel<string>('metaVersion', { required: true })
const metaDescription = defineModel<string>('metaDescription', { required: true })
const shadowEnabled = defineModel<boolean>('shadowEnabled', { required: true })
const hotspot = defineModel<Hotspot>('hotspot', { required: true })
const perSizeHotspot = defineModel<boolean>('perSizeHotspot', { required: true })

interface ExportProgress {
  buildId: string
  stage: 'role' | 'package' | 'sign' | 'done' | 'cancelled' | 'error'
  current: number
  total: number
  message: string | null
}

const props = defineProps<{
  arrowAssigned: boolean
  assignedRoleCount: number
  exportMessage: string | null
  exportProgress: ExportProgress | null
  exportBusy: boolean
  activeRoleJp: string
  showAdvancedResolutions: boolean
  failedApplyThemeId: string | null
  /** px ⇔ ratio 変換の基準サイズ */
  primarySize: number
  /** 現在のサイズに sized.hotspot override が有効かどうか */
  sizedOverrideActive: boolean
  /** enableSizedOverride ボタンを押せる条件 (アセット割当済み + perSizeHotspot=ON) */
  canEditSizedOverride: boolean
}>()

/** px 入力 ⇔ ratio model の双方向ブリッジ */
const hotspotXPx = computed<number>({
  get: () => Math.round(hotspot.value.x * (props.primarySize || 1)),
  set: (px: number) => {
    const size = props.primarySize || 1
    const x = Math.max(0, Math.min(1, px / size))
    hotspot.value = { ...hotspot.value, x }
  },
})

const hotspotYPx = computed<number>({
  get: () => Math.round(hotspot.value.y * (props.primarySize || 1)),
  set: (px: number) => {
    const size = props.primarySize || 1
    const y = Math.max(0, Math.min(1, px / size))
    hotspot.value = { ...hotspot.value, y }
  },
})

defineEmits<{
  (e: 'dismiss-export-message'): void
  (e: 'cancel-export'): void
  (e: 'retry-apply'): void
  (e: 'enable-sized-override'): void
}>()
</script>

<template>
  <div class="metadata-pane">
    <div class="metadata-grid">
      <div class="prop-section">
        <div class="prop-head">{{ t('creator.metaTitle') }}</div>
        <div class="prop-body" style="padding: 4px 16px">
          <SettingsRow :label="t('creator.metaNameJa')" :desc="t('creator.metaNameJaDesc')">
            <input v-model="metaName" class="input" style="width: 280px" placeholder="Neon Glow" />
          </SettingsRow>
          <SettingsRow :label="t('creator.metaNameEn')" :desc="t('creator.metaNameEnDesc')">
            <input
              v-model="metaNameEn"
              class="input"
              style="width: 280px"
              placeholder="Neon Glow"
            />
          </SettingsRow>
          <SettingsRow :label="t('creator.metaAuthor')" :desc="t('creator.metaAuthorDesc')">
            <input
              v-model="metaAuthor"
              class="input"
              style="width: 280px"
              placeholder="@username"
            />
          </SettingsRow>
          <SettingsRow :label="t('creator.metaVersion')" :desc="t('creator.metaVersionDesc')">
            <input
              v-model="metaVersion"
              class="input mono"
              style="width: 140px"
              placeholder="1.0.0"
            />
          </SettingsRow>
          <SettingsRow :label="t('creator.metaShadow')" :desc="t('creator.metaShadowDesc')">
            <SettingsToggle v-model="shadowEnabled" />
          </SettingsRow>
        </div>
      </div>

      <div class="prop-section">
        <div class="prop-head">{{ t('creator.metaDescTitle') }}</div>
        <div class="prop-body" style="padding: 12px 16px">
          <textarea
            v-model="metaDescription"
            class="input"
            rows="6"
            style="width: 100%; font-family: var(--font-body); resize: vertical"
            :placeholder="t('creator.metaDescPlaceholder')"
          />
        </div>
      </div>

      <div class="prop-section">
        <div class="prop-head">{{ t('creator.metaExportStatus') }}</div>
        <div class="prop-body" style="padding: 4px 16px">
          <SettingsRow :label="t('creator.metaAssignedRoles')">
            <span class="tag" :class="{ ok: arrowAssigned }">{{ assignedRoleCount }} / 17</span>
          </SettingsRow>
          <SettingsRow :label="t('creator.metaArrowRequired')">
            <span class="tag" :class="arrowAssigned ? 'ok' : ''">
              {{ arrowAssigned ? t('creator.metaAssigned') : t('creator.metaUnassigned') }}
            </span>
          </SettingsRow>
        </div>
      </div>

      <div class="prop-section">
        <div class="prop-head">
          {{ t('creator.metaHotspotTitle') }}
          <span class="role-tag">{{ activeRoleJp }}</span>
        </div>
        <div class="prop-body" style="padding: 4px 16px">
          <SettingsRow :label="t('creatorStart.propHotspotX')">
            <input
              v-model.number="hotspotXPx"
              type="number"
              class="input mono"
              min="0"
              :max="primarySize"
              style="width: 120px"
            />
          </SettingsRow>
          <SettingsRow :label="t('creatorStart.propHotspotY')">
            <input
              v-model.number="hotspotYPx"
              type="number"
              class="input mono"
              min="0"
              :max="primarySize"
              style="width: 120px"
            />
          </SettingsRow>
          <SettingsRow v-if="showAdvancedResolutions" :label="t('creatorStart.propPerSize')">
            <SettingsToggle v-model="perSizeHotspot" />
          </SettingsRow>
          <SettingsRow
            v-if="showAdvancedResolutions && perSizeHotspot"
            :label="t('creator.perSizeOverride')"
          >
            <button
              class="btn"
              :disabled="!canEditSizedOverride || sizedOverrideActive"
              @click="$emit('enable-sized-override')"
            >
              {{ sizedOverrideActive ? t('creator.perSizeActive') : t('creator.perSizeEnable') }}
            </button>
          </SettingsRow>
        </div>
      </div>
    </div>

    <Transition name="fade">
      <div v-if="exportMessage" class="import-banner" role="status">
        <UiIcon
          :name="exportMessage.startsWith('エクスポート失敗') ? 'Alert' : 'Check'"
          :size="13"
        />
        <span>{{ exportMessage }}</span>
        <button
          v-if="failedApplyThemeId"
          class="btn ghost"
          style="height: 24px; margin-left: auto"
          @click="$emit('retry-apply')"
        >
          {{ t('saveModal.retryApply') }}
        </button>
        <button
          class="btn ghost"
          :style="failedApplyThemeId ? 'height: 24px' : 'margin-left: auto; height: 24px'"
          @click="$emit('dismiss-export-message')"
        >
          <UiIcon name="X" :size="11" />
        </button>
      </div>
    </Transition>

    <!-- ストリームエクスポート中の進捗バー + キャンセルボタン -->
    <Transition name="fade">
      <div
        v-if="exportProgress && exportProgress.stage !== 'done'"
        class="export-progress"
        role="status"
        aria-live="polite"
      >
        <div class="export-progress-row">
          <span class="export-progress-label">
            <template v-if="exportProgress.stage === 'role'">
              {{ exportProgress.message ?? '' }} ({{ exportProgress.current }}/{{
                exportProgress.total
              }})
            </template>
            <template v-else-if="exportProgress.stage === 'sign'">{{
              t('creatorStart.exportStageSign')
            }}</template>
            <template v-else-if="exportProgress.stage === 'package'">{{
              t('creatorStart.exportStagePackage')
            }}</template>
            <template v-else-if="exportProgress.stage === 'cancelled'">{{
              t('creatorStart.exportStageCancelled')
            }}</template>
            <template v-else>{{ t('creatorStart.exportStageWorking') }}</template>
          </span>
          <button
            v-if="exportBusy && exportProgress.stage !== 'cancelled'"
            class="btn ghost"
            style="height: 24px; margin-left: auto"
            @click="$emit('cancel-export')"
          >
            <UiIcon name="X" :size="11" />キャンセル
          </button>
        </div>
        <div class="export-progress-bar">
          <div
            class="export-progress-fill"
            :style="{
              width:
                exportProgress.total > 0
                  ? `${(exportProgress.current / exportProgress.total) * 100}%`
                  : '0%',
            }"
          />
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
/* NOTE: 元の scoped は --border / --bg-elev1 / --bg-elev2 / --text / --text-mute /
 * --mint などの未定義トークンに依存しており、それらの declaration は invalid と
 * なって cascade で global.css の .prop-section / .prop-head / .input / .tag /
 * .btn ルールが見た目を提供していた。
 * @apply の border utility が global を上書きする問題を避けるため、scoped 側は
 * global と衝突しないレイアウト/スペーシングの差分のみを CSS リテラルで保持する。
 * .metadata-pane / .metadata-grid / .import-banner / .export-progress* / .role-tag
 * は global に同名ルールが無いコンポーネント固有のものなのでそのまま残す。 */

.metadata-pane {
  display: grid;
  gap: 16px;
  padding: 16px;
}

.metadata-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 12px;
}

@media (min-width: 900px) {
  .metadata-grid {
    grid-template-columns: 1fr 1fr;
  }

  .metadata-grid > .prop-section:nth-child(1),
  .metadata-grid > .prop-section:nth-child(2) {
    grid-column: span 2;
  }
}

.prop-section {
  border-radius: 12px;
}

.prop-head {
  padding: 10px 16px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
}

.input {
  height: 32px;
  border-radius: 8px;
  padding: 0 10px;
  font-size: 13px;
}

.input.mono {
  font-family: var(--font-mono);
}

.tag {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  gap: 4px;
}

.tag.ok {
  border-color: rgba(106, 213, 184, 0.3);
}

.import-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-radius: 10px;
  background: var(--bg-elev2);
  border: 1px solid var(--border);
  font-size: 12px;
  color: var(--text);
}

.export-progress {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px 14px;
  background: var(--bg-elev2);
  display: grid;
  gap: 8px;
}

.export-progress-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.export-progress-label {
  font-size: 12px;
  color: var(--text-mute);
}

.export-progress-bar {
  height: 4px;
  border-radius: 2px;
  background: var(--bg-elev1);
  overflow: hidden;
}

.export-progress-fill {
  height: 100%;
  background: var(--mint);
  transition: width 200ms ease;
}

.btn {
  padding: 0 14px;
  border-radius: 8px;
  font-size: 13px;
}

.btn.ghost {
  background: transparent;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 200ms ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.role-tag {
  margin-left: 8px;
  font-size: 11px;
  color: var(--text-mute);
  font-family: var(--font-mono);
  font-weight: 400;
  text-transform: none;
}
</style>
