<script setup lang="ts">
/**
 * 同 ID のテーマを再インポートしようとした際の比較ダイアログ。
 * ユーザーに「上書き / バージョン併存 / キャンセル」の 3 択を提示する。
 *
 * `cursorpack` に同梱されている theme.json の内容と、
 * 既存テーマの theme.json を並べて表示。
 */

const { t } = useI18n()

export interface ConflictInfo {
  id: string
  name: string
  version: string
  author: string | null
  roleCount: number
  existing: {
    name: string
    version: string
    author: string | null
    roleCount: number
  }
}

const props = defineProps<{
  info: ConflictInfo
}>()

const emit = defineEmits<{
  /** 上書きインポート */
  overwrite: []
  /** キャンセル */
  cancel: []
}>()

/** `1.2.3` → `[1, 2, 3]` */
function parseSemver(v: string): number[] {
  return v.split('.').map((p) => Number.parseInt(p, 10) || 0)
}

const compareResult = computed(() => {
  const a = parseSemver(props.info.version)
  const b = parseSemver(props.info.existing.version)
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const da = a[i] ?? 0
    const db = b[i] ?? 0
    if (da > db) return 'newer' as const
    if (da < db) return 'older' as const
  }
  return 'same' as const
})

const headerLabel = computed(() => {
  switch (compareResult.value) {
    case 'newer':
      return t('conflict.headerNewer')
    case 'older':
      return t('conflict.headerOlder')
    default:
      return t('conflict.headerSame')
  }
})

const headerDesc = computed(() => {
  switch (compareResult.value) {
    case 'newer':
      return t('conflict.descNewer')
    case 'older':
      return t('conflict.descOlder')
    default:
      return t('conflict.descSame')
  }
})

const headerAccent = computed(() =>
  compareResult.value === 'older' ? 'var(--rose)' : 'var(--accent)',
)

function onBackdrop(e: MouseEvent) {
  if (e.target === e.currentTarget) emit('cancel')
}
</script>

<template>
  <div
    class="modal-page"
    role="dialog"
    aria-modal="true"
    aria-labelledby="conflict-modal-title"
    @click="onBackdrop"
  >
    <div class="modal conflict-modal" @click.stop>
      <div class="modal-head">
        <div
          class="modal-icon"
          aria-hidden="true"
          :style="{
            borderColor: `${headerAccent}59`,
            color: headerAccent,
            background: `${headerAccent}1f`,
          }"
        >
          <UiIcon name="Alert" :size="20" />
        </div>
        <div style="flex: 1; min-width: 0">
          <h2 id="conflict-modal-title">{{ headerLabel }}</h2>
          <p>{{ headerDesc }}</p>
        </div>
      </div>

      <div class="modal-body">
        <div class="diff-grid">
          <div class="col">
            <div class="col-head">{{ t('conflict.colExisting') }}</div>
            <div class="diff-name">{{ info.existing.name }}</div>
            <div class="diff-meta">
              <span class="m">v{{ info.existing.version }}</span>
              <span class="m">@{{ info.existing.author ?? 'unknown' }}</span>
              <span class="m">{{ info.existing.roleCount }}/17</span>
            </div>
          </div>

          <div class="arrow" :style="{ color: headerAccent }">
            <svg
              width="22"
              height="22"
              viewBox="0 0 22 22"
              fill="none"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M3 11h16M14 5l5 6-5 6" />
            </svg>
          </div>

          <div class="col" :style="{ borderColor: headerAccent }">
            <div class="col-head" :style="{ color: headerAccent }">
              {{ t('conflict.colImport') }}
            </div>
            <div class="diff-name">{{ info.name }}</div>
            <div class="diff-meta">
              <span class="m" :style="{ color: headerAccent }">v{{ info.version }}</span>
              <span class="m">@{{ info.author ?? 'unknown' }}</span>
              <span class="m">{{ info.roleCount }}/17</span>
            </div>
          </div>
        </div>

        <div class="id-row">
          <UiIcon name="Pkg" :size="13" />
          <span class="id-label">theme.id</span>
          <span class="id-value">{{ info.id }}</span>
        </div>
      </div>

      <div class="modal-foot">
        <div class="left-note">
          <UiIcon name="Shield" :size="12" style="color: var(--accent)" />
          {{ t('conflict.snapshotNote') }}
        </div>
        <div class="actions">
          <button class="btn ghost" @click="emit('cancel')">{{ t('common.cancel') }}</button>
          <button
            :class="['btn', compareResult === 'older' ? 'danger' : 'primary']"
            @click="emit('overwrite')"
          >
            <UiIcon name="Check" :size="13" />
            {{ compareResult === 'older' ? t('conflict.downgrade') : t('conflict.overwrite') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.conflict-modal {
  @apply w-[580px];
}

.diff-grid {
  @apply grid grid-cols-[1fr_auto_1fr] items-stretch gap-3.5;
}
.col {
  @apply rounded-[10px] border border-line bg-white/[0.02] p-3.5;
}
.col-head {
  @apply mb-1.5 font-mono text-[9.5px] uppercase tracking-[0.16em] text-fg-mute;
}
.diff-name {
  @apply mb-1.5 font-display text-[14px] font-semibold tracking-[-0.01em];
}
.diff-meta {
  @apply flex items-center gap-0 font-mono text-[10.5px] text-fg-mute;
}
.diff-meta .m {
  @apply px-2;
}
.diff-meta .m:first-child {
  @apply pl-0;
}
.diff-meta .m + .m {
  @apply border-l border-line;
}

.arrow {
  @apply grid place-items-center px-1;
}

.id-row {
  @apply mt-3.5 flex items-center gap-2.5 rounded-[8px] border border-line bg-black/20 px-3 py-2.5 font-mono text-[11px] text-fg-dim;
}
.id-label {
  @apply text-fg-mute;
}
.id-value {
  @apply break-all text-fg;
}
</style>
