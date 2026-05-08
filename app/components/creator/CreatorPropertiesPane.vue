<script setup lang="ts">
/**
 * Creator の右カラム — プロパティパネル (Hotspot / Asset / Validation 3 セクション)。
 *
 * v-model バインディング 4 個 + read-only 3 個。assign タブの右側に固定表示される。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const hotspotX = defineModel<number>('hotspotX', { required: true })
const hotspotY = defineModel<number>('hotspotY', { required: true })
const perSizeHotspot = defineModel<boolean>('perSizeHotspot', { required: true })
const shadowEnabled = defineModel<boolean>('shadowEnabled', { required: true })

defineProps<{
  showAdvancedResolutions: boolean
  importedPreviewUrl: string | null
  sanitizedRemovals: string[]
  resample: 'lanczos' | 'nearest' | 'auto'
}>()
</script>

<template>
  <div class="cpane right">
    <!-- Hotspot -->
    <div class="prop-section">
      <div class="prop-head">
        {{ t('creator.propsHotspot') }}
        <span class="tag ok">px</span>
      </div>
      <div class="prop-body">
        <div class="prop-row">
          <label>{{ t('creatorStart.propHotspotX') }}</label>
          <input v-model.number="hotspotX" type="number" class="input mono" min="0" />
        </div>
        <div class="prop-row">
          <label>{{ t('creatorStart.propHotspotY') }}</label>
          <input v-model.number="hotspotY" type="number" class="input mono" min="0" />
        </div>
        <div v-if="showAdvancedResolutions" class="prop-row">
          <label>{{ t('creatorStart.propPerSize') }}</label>
          <button
            :class="['toggle', { on: perSizeHotspot }]"
            :aria-pressed="perSizeHotspot"
            @click="perSizeHotspot = !perSizeHotspot"
          >
            <span class="knob" />
          </button>
        </div>
      </div>
    </div>

    <!-- アセット -->
    <div class="prop-section">
      <div class="prop-head">{{ t('creator.propsAsset') }}</div>
      <div class="prop-body">
        <div class="prop-row">
          <label>{{ t('creatorStart.propAssetFormat') }}</label>
          <span class="tag">PNG · 24bit · α</span>
        </div>
        <div class="prop-row">
          <label>{{ t('creatorStart.propAssetColor') }}</label>
          <div class="color-chips">
            <span class="cc" style="background: #7cf2d4" />
            <span class="cc" style="background: #0a0b0f" />
            <span class="cc" style="background: #ffffff" />
          </div>
        </div>
        <div class="prop-row">
          <label>{{ t('creatorStart.propAssetShadow') }}</label>
          <button
            :class="['toggle', { on: shadowEnabled }]"
            :aria-pressed="shadowEnabled"
            @click="shadowEnabled = !shadowEnabled"
          >
            <span class="knob" />
          </button>
        </div>
      </div>
    </div>

    <!-- Validation -->
    <div class="prop-section">
      <div class="prop-head">
        {{ t('creator.propsValidation') }}
        <span class="tag ok"><UiIcon name="Check" :size="10" />pass</span>
      </div>
      <div class="prop-body validation-body">
        <div class="vrow">
          <span>magic-byte</span>
          <span :class="importedPreviewUrl ? 'ok' : 'dim'">
            {{ importedPreviewUrl ? 'OK' : '—' }}
          </span>
        </div>
        <div class="vrow">
          <span>svg-sanitize</span>
          <span :class="sanitizedRemovals.length === 0 ? 'ok' : 'warn'">
            {{ sanitizedRemovals.length === 0 ? 'clean' : `removed ${sanitizedRemovals.length}` }}
          </span>
        </div>
        <div class="vrow">
          <span>resample-strategy</span><span class="dim">{{ resample }}3</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cpane.right {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  border-left: 1px solid var(--border);
  overflow-y: auto;
}

.prop-section {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-elev1);
}

.prop-head {
  padding: 10px 16px;
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-mute);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 6px;
}

.prop-body {
  padding: 8px 14px;
  display: grid;
  gap: 10px;
}

.prop-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.prop-row label {
  font-size: 12px;
  color: var(--text-mute);
}

.input {
  height: 32px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  color: var(--text);
  padding: 0 10px;
  font-size: 13px;
  width: 90px;
  text-align: right;
}

.input.mono {
  font-family: var(--font-mono);
}

.tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  font-size: 11px;
  color: var(--text-mute);
}

.tag.ok {
  color: var(--mint);
  border-color: rgba(106, 213, 184, 0.3);
}

.toggle {
  width: 36px;
  height: 20px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  cursor: pointer;
  position: relative;
  padding: 0;
}

.toggle .knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--text-mute);
  transition: transform 150ms ease;
}

.toggle.on {
  background: rgba(106, 213, 184, 0.2);
  border-color: var(--mint);
}

.toggle.on .knob {
  transform: translateX(16px);
  background: var(--mint);
}

.color-chips {
  display: flex;
  gap: 4px;
}

.cc {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 1px solid var(--border);
}

.validation-body .vrow {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  font-family: var(--font-mono);
}

.validation-body .ok {
  color: var(--mint);
}

.validation-body .warn {
  color: var(--rose);
}

.validation-body .dim {
  color: var(--text-mute);
}
</style>
