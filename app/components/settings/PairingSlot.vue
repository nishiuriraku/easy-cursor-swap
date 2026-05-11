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
@reference '~/assets/css/tailwind.css';

.pairing-slot {
  @apply p-0;
}
.pairing-slot.current {
  border-color: color-mix(in srgb, var(--pair-accent) 33%, transparent);
}

.pairing-head {
  @apply flex items-center justify-between border-b border-line px-4 py-3;
}
.pairing-title {
  @apply flex items-center gap-2;
}
.pair-dot {
  @apply size-2 rounded-full;
  background: var(--pair-accent);
  box-shadow: 0 0 8px var(--pair-accent);
}
.pair-label {
  @apply font-display text-[13px] font-semibold;
}
.pair-sub {
  @apply mt-0.5 font-mono text-[10px] text-fg-mute;
}

.small-tag {
  @apply px-1.5 py-px text-[9px];
}

.change-btn {
  @apply h-[26px] text-[11px];
}
</style>
