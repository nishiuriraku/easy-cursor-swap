<script setup lang="ts">
/**
 * Light/Dark テーマペアリングカード。
 * カバレッジバーの色は accent プロップで切替 (light=amber, dark=mint)。
 */
import { computed } from 'vue'

interface PairingTheme {
  name: string
  author: string
  version: string
  includedRoles: string[]
}

const props = defineProps<{
  label: string
  sub: string
  theme: PairingTheme
  /** バーのアクセント色 (CSS) */
  accent: string
  /** OS の現在状態と一致するなら true */
  current?: boolean
}>()

defineEmits<{
  change: []
}>()

const coveragePct = computed(() => Math.round((props.theme.includedRoles.length / 17) * 100))
</script>

<template>
  <div :class="['card', 'pairing-slot', { current }]" :style="{ '--pair-accent': accent }">
    <div class="pairing-head">
      <div>
        <div class="pairing-title">
          <span class="pair-dot" />
          <span class="pair-label">{{ label }}</span>
          <span v-if="current" class="tag ok small-tag">ACTIVE</span>
        </div>
        <div class="pair-sub">{{ sub }}</div>
      </div>
    </div>
    <div class="card-preview">
      <CursorMatrix :included="theme.includedRoles" />
    </div>
    <div class="card-body">
      <div class="card-row">
        <div>
          <div class="card-name">{{ theme.name }}</div>
          <div class="card-author">@{{ theme.author }} · v{{ theme.version }}</div>
        </div>
        <button class="btn ghost change-btn" @click="$emit('change')">
          変更<UiIcon name="ChevD" :size="11" />
        </button>
      </div>
      <div class="coverage">
        <div class="bar">
          <i
            :style="{
              width: coveragePct + '%',
              background: `linear-gradient(90deg, ${accent}, ${accent}cc)`,
            }"
          />
        </div>
        <span class="num">{{ theme.includedRoles.length }}/17</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pairing-slot {
  padding: 0;
}
.pairing-slot.current {
  border-color: color-mix(in srgb, var(--pair-accent) 33%, transparent);
}

.pairing-head {
  padding: 12px 16px;
  border-bottom: 1px solid var(--line);
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.pairing-title {
  display: flex;
  align-items: center;
  gap: 8px;
}
.pair-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--pair-accent);
  box-shadow: 0 0 8px var(--pair-accent);
}
.pair-label {
  font-family: var(--font-display);
  font-weight: 600;
  font-size: 13px;
}
.pair-sub {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--fg-mute);
  margin-top: 2px;
}

.small-tag {
  padding: 1px 6px;
  font-size: 9px;
}

.change-btn {
  height: 26px;
  font-size: 11px;
}
</style>
