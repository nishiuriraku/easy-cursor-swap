<script setup lang="ts">
/**
 * 同 ID のテーマを再インポートしようとした際の比較ダイアログ。
 * ユーザーに「上書き / バージョン併存 / キャンセル」の 3 択を提示する。
 *
 * `cursorpack` に同梱されている theme.json の内容と、
 * 既存テーマの theme.json を並べて表示。
 */
import { computed } from 'vue'
import { useI18n } from '~/composables/useI18n'

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
  <div class="modal-page" role="dialog" aria-modal="true" @click="onBackdrop">
    <div class="modal conflict-modal" @click.stop>
      <div class="modal-head">
        <div class="modal-icon" :style="{ borderColor: `${headerAccent}59`, color: headerAccent, background: `${headerAccent}1f` }">
          <UiIcon name="Alert" :size="20" />
        </div>
        <div style="flex: 1; min-width: 0">
          <h2>{{ headerLabel }}</h2>
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
            <svg width="22" height="22" viewBox="0 0 22 22" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 11h16M14 5l5 6-5 6" />
            </svg>
          </div>

          <div class="col" :style="{ borderColor: headerAccent }">
            <div class="col-head" :style="{ color: headerAccent }">{{ t('conflict.colImport') }}</div>
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
.conflict-modal { width: 580px; }

.diff-grid {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  gap: 14px;
  align-items: stretch;
}
.col {
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 14px;
}
.col-head {
  font-family: var(--font-mono);
  font-size: 9.5px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--fg-mute);
  margin-bottom: 6px;
}
.diff-name {
  font-family: var(--font-display);
  font-size: 14px;
  font-weight: 600;
  letter-spacing: -0.01em;
  margin-bottom: 6px;
}
.diff-meta {
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--fg-mute);
  display: flex;
  align-items: center;
  gap: 0;
}
.diff-meta .m { padding: 0 8px; }
.diff-meta .m:first-child { padding-left: 0; }
.diff-meta .m + .m { border-left: 1px solid var(--line); }

.arrow {
  display: grid;
  place-items: center;
  padding: 0 4px;
}

.id-row {
  margin-top: 14px;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  display: flex;
  align-items: center;
  gap: 10px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--fg-dim);
}
.id-label { color: var(--fg-mute); }
.id-value { color: var(--fg); word-break: break-all; }
</style>
