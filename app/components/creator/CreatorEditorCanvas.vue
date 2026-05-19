<script setup lang="ts">
/**
 * Creator の Assign タブの中央エディタペイン (右側 1 カラム) を担う子コンポーネント。
 *
 * 役割タイトル + ビッグプレビュー (ホットスポット編集) + 詳細設定パネル (SizeStrip +
 * リサンプル切替) + 「メタデータに進む」ボタン。
 *
 * 親 (creator.vue) は activeRole / size / preview-asset / hotspot を props で渡し、
 * ユーザー操作 (hotspot 更新 / サイズ選択 / 詳細トグル) は emit で受ける。
 * 編集ロジック (writeActiveHotspot, perSizeHotspot 切替, sizedOverride 等) は親に残す。
 */
import type { CursorRoleDef } from '~/components/icons/CursorIcons'
import type { CursorPreviewAsset } from '~/components/preview/CursorPreview.vue'
import type { Hotspot } from '~/composables/useCreatorAssets'

type ResampleMode = 'lanczos' | 'nearest'

const SIZES = [32, 48, 64, 96, 128, 256] as const

const props = defineProps<{
  activeRole: CursorRoleDef
  activeSize: number
  previewAsset: CursorPreviewAsset
  hotspot: Hotspot
  referenceSize: number
  filledSizes: number[]
  sizePreviewMap: Record<number, string>
  arrowAssigned: boolean
  isRequiredRole: boolean
  showAdvancedResolutions: boolean
  resample: ResampleMode
}>()

const emit = defineEmits<{
  'update:showAdvancedResolutions': [v: boolean]
  'update:resample': [v: ResampleMode]
  'update:hotspot': [h: Hotspot]
  'select-size': [s: number]
  'file-selected': [e: Event]
  'next-tab': []
}>()

const { t } = useI18n()

function centerHotspot() {
  emit('update:hotspot', { x: 0.5, y: 0.5 })
}
</script>

<template>
  <div class="editor">
    <div class="editor-head">
      <div>
        <h2>
          {{ props.activeRole.jp }}
          <span class="role-key">{{ props.activeRole.id }}</span>
        </h2>
        <div class="desc">
          <template v-if="props.isRequiredRole">
            {{ t('creator.requiredRoleNote').split('{required}')[0]
            }}<b style="color: var(--accent)">{{ t('creator.requiredMark') }}</b
            >{{ t('creator.requiredRoleNote').split('{required}')[1] }}
          </template>
          <template v-else>
            {{ t('creator.optionalRoleNote', { en: props.activeRole.en }) }}
          </template>
        </div>
      </div>
      <div style="display: flex; gap: 6px">
        <input
          type="file"
          accept=".png,.svg,image/png,image/svg+xml"
          hidden
          @change="emit('file-selected', $event)"
        />
      </div>
    </div>

    <div class="canvas-area">
      <div class="canvas-stage">
        <!-- ビッグプレビュー (CursorPreview に委譲、メタコーナーは外側でオーバーレイ) -->
        <div class="bigpreview-wrapper" :title="t('creator.hotspotHint')">
          <CursorPreview
            :asset="props.previewAsset"
            :hotspot="props.hotspot"
            :display-pct="70"
            editable
            :reference-px="props.referenceSize"
            :hide-dot="props.previewAsset.kind === 'empty'"
            class="bigpreview"
            @update:hotspot="(h: Hotspot) => emit('update:hotspot', h)"
          />
          <div class="preview-meta tl">{{ props.activeSize }} × {{ props.activeSize }}</div>
          <button
            class="hotspot-center-btn"
            :title="t('creator.centerHotspot')"
            @pointerdown.stop
            @click.stop="centerHotspot"
          >
            <UiIcon name="Crosshair" :size="11" />
          </button>
          <div class="preview-meta tr">
            hotspot {{ props.hotspot.x.toFixed(3) }},{{ props.hotspot.y.toFixed(3) }}
          </div>
        </div>

        <!-- 詳細設定トグル: 解像度別ワークフローを ON/OFF -->
        <div class="advanced-toggle-row">
          <button
            class="advanced-toggle"
            :aria-expanded="props.showAdvancedResolutions"
            @click="emit('update:showAdvancedResolutions', !props.showAdvancedResolutions)"
          >
            <UiIcon
              name="ChevD"
              :size="11"
              :style="{
                transform: props.showAdvancedResolutions ? 'rotate(0deg)' : 'rotate(-90deg)',
                transition: 'transform 160ms',
              }"
            />
            {{ t('creator.advancedSection') }}
            <span class="advanced-hint">{{ t('creator.advancedHint') }}</span>
          </button>

          <!-- リサンプル切替 (基本フローでも見せておく) -->
          <div class="resample-row">
            <span>RESAMPLE</span>
            <div class="btn-group">
              <button
                v-for="mode in ['lanczos', 'nearest'] as ResampleMode[]"
                :key="mode"
                :class="['btn', { active: props.resample === mode }]"
                style="height: 26px; font-size: 11px"
                @click="emit('update:resample', mode)"
              >
                {{ mode === 'lanczos' ? 'Lanczos' : 'Nearest' }}
              </button>
            </div>
          </div>
        </div>

        <!-- 詳細設定: 解像度別の上書きワークフロー -->
        <Transition name="fade">
          <div v-if="props.showAdvancedResolutions" class="advanced-panel">
            <div class="advanced-label">{{ t('creator.perResolutionLabel') }}</div>
            <SizeStrip
              :sizes="[...SIZES]"
              :filled-sizes="props.filledSizes"
              :active-size="props.activeSize"
              :role="props.activeRole.id"
              :preview-map="props.sizePreviewMap"
              @select="(s: number) => emit('select-size', s)"
            />
          </div>
        </Transition>

        <div class="next-step-row">
          <button
            class="btn primary"
            :disabled="!props.arrowAssigned"
            :title="!props.arrowAssigned ? t('creator.requiredMark') : ''"
            @click="emit('next-tab')"
          >
            {{ t('creator.nextToMetadata') }}
            <UiIcon name="ChevD" :size="13" :style="{ transform: 'rotate(-90deg)' }" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.role-key {
  @apply ml-2 font-mono text-[12px] font-normal text-fg-mute;
}

.preview-meta {
  @apply absolute bottom-2.5 font-mono text-[10px];
}
.preview-meta.tl {
  @apply left-3 text-fg-mute;
}
.preview-meta.tr {
  @apply right-3 text-accent;
}

.hotspot-center-btn {
  @apply absolute right-2 top-2 inline-flex size-[22px] cursor-pointer items-center justify-center rounded-full border border-accent-line p-0 text-accent;
  background: rgba(0, 0, 0, 0.4);
  transition:
    background 120ms ease,
    transform 120ms ease;
}
:where(html.light) .hotspot-center-btn {
  background: rgba(15, 168, 133, 0.08);
}
.hotspot-center-btn:hover {
  background: rgba(124, 242, 212, 0.15);
  transform: scale(1.08);
}
:where(html.light) .hotspot-center-btn:hover {
  background: rgba(15, 168, 133, 0.18);
}

.next-step-row {
  /* NOTE: var(--border) は未定義 (元コード leftover) — 結果として border 無し。
   * 視覚的現状維持のためそのまま literal で残置。 */
  @apply mt-4 flex justify-end pt-4;
  border-top: 1px solid var(--border);
}
.next-step-row .btn.primary {
  @apply h-9 px-[18px] text-[13px];
}

.resample-row {
  @apply flex items-center gap-2 font-mono text-[10.5px] tracking-[0.04em] text-fg-mute;
}

.advanced-toggle-row {
  @apply flex flex-wrap items-center justify-between gap-4;
}
.advanced-toggle {
  @apply inline-flex cursor-pointer items-center gap-1.5 rounded-[8px] border border-dashed border-line bg-transparent px-2.5 py-1.5 font-mono text-[11.5px] tracking-[0.04em] text-fg-mute;
  transition:
    color 160ms ease,
    border-color 160ms ease;
}
.advanced-toggle:hover {
  @apply border-accent-line text-fg;
}
.advanced-toggle[aria-expanded='true'] {
  @apply border-accent-line text-accent;
}
.advanced-hint {
  @apply ml-1 font-body text-[10.5px] text-fg-dim;
}

.advanced-panel {
  @apply mt-2 rounded-[8px] border border-line px-3 py-2.5;
  background: rgba(124, 242, 212, 0.02);
}
.advanced-label {
  @apply mb-2 font-mono text-[10px] uppercase tracking-[0.16em] text-fg-mute;
}

.editor {
  @apply flex min-w-0 flex-col;
  background: radial-gradient(800px 600px at 50% 0%, rgba(124, 242, 212, 0.04), transparent 60%);
}
.editor-head {
  @apply flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-line px-[18px] py-3.5;
}
.editor-head h2 {
  @apply m-0 font-display text-[18px] font-semibold tracking-[-0.01em];
}
.editor-head .desc {
  @apply mt-0.5 text-[12px] text-fg-dim;
}
.canvas-area {
  @apply grid min-h-0 flex-1 overflow-auto p-6;
  grid-template-columns: 1fr;
}
.canvas-stage {
  @apply flex flex-col items-center gap-[18px];
  align-self: center;
  justify-self: center;
}

.bigpreview-wrapper {
  @apply relative grid size-[220px] min-h-0 place-items-center rounded-2xl border border-line-hi;
  background:
    repeating-conic-gradient(rgba(255, 255, 255, 0.025) 0% 25%, transparent 0% 50%) 0 / 18px 18px,
    var(--bg-1);
  box-shadow: var(--shadow-2);
}
.bigpreview {
  @apply size-full;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 200ms ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
