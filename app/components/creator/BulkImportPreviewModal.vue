<script setup lang="ts">
/**
 * バルクインポート / .cursorpack 取り込みのプレビュー & ロール割当モーダル。
 *
 * 割当ライフサイクル (matches / unmatched の三方移動, watch による初期化, apply payload
 * 組立) は `useBulkImportPreviewState` composable に分離済み。本 SFC は presentation
 * (テンプレート + 派生 computed + apply emit) に専念する。
 */
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import type { ApplyPayload } from '~/composables/useBulkImportPreviewState'
import type { ResolvedAsset, ParsedCursorpack } from '~/composables/useBulkImport'

const { t } = useI18n()

interface Props {
  open: boolean
  /** 通常のバルクインポート結果。.cursorpack 経路では null。 */
  resolved: ResolvedAsset[] | null
  /** .cursorpack 経路。それ以外では null。 */
  cursorpack: ParsedCursorpack | null
  /** 既に割り当て済みのロール (上書き判定用)。 */
  existingRoles: Set<string>
  sourceLabel: string
}
const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'apply', payload: ApplyPayload): void
  (e: 'cancel'): void
}>()

const {
  matches,
  unmatched,
  metadataChoice,
  unassignRole,
  pickRoleFromUnmatched,
  unassignAll,
  buildApplyPayload,
} = useBulkImportPreviewState({
  open: () => props.open,
  resolved: () => props.resolved,
  cursorpack: () => props.cursorpack,
  existingRoles: () => props.existingRoles,
})

const summaryLine = computed(() => {
  const auto = matches.value.length
  const conflicts = matches.value.filter((m) => m.conflict !== 'none').length
  return t('bulkImport.previewSummary', { auto, unmatched: unmatched.value.length, conflicts })
})

const allRoleRows = computed(() => {
  const byRole = new Map(matches.value.map((m) => [m.role, m]))
  return CURSOR_ROLE_IDS.map((roleId) => {
    const m = byRole.get(roleId)
    const def = CURSOR_ROLES.find((r) => r.id === roleId)
    return {
      roleId,
      roleLabel: def?.jp ?? roleId,
      required: roleId === 'Arrow',
      match: m,
    }
  })
})

/**
 * 未マッチプールの表示順。ファイル名で自然数順 (`1.cur < 2.cur < 10.cur`) に固定。
 * `Intl.Collator({ numeric: true })` で日本語ファイル名も含めて自然な辞書順に並ぶ。
 */
const naturalCollator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })
const sortedUnmatched = computed(() =>
  [...unmatched.value].sort((a, b) =>
    naturalCollator.compare(a.asset.sourceFile, b.asset.sourceFile),
  ),
)

/**
 * 未マッチ行ドロップダウンの選択肢。
 * 空きロールを先頭、割当済ロールを末尾に並べ替え、割当済には「上書き」サフィックスを付ける。
 * 割当済を選ぶと `pickRoleFromUnmatched` 内で既存ファイルが未マッチに戻る (swap)。
 */
const unmatchedRoleOptions = computed(() => {
  const assigned = new Set(matches.value.map((m) => m.role))
  const placeholder = { value: null, label: t('bulkImport.selectRolePlaceholder') }
  const empty: Array<{ value: string; label: string }> = []
  const taken: Array<{ value: string; label: string }> = []
  for (const r of CURSOR_ROLE_IDS) {
    const def = CURSOR_ROLES.find((d) => d.id === r)
    const base = def ? `${def.jp}（${r}）` : r
    if (assigned.has(r)) {
      taken.push({ value: r, label: `${base} — ${t('bulkImport.alreadyAssignedSuffix')}` })
    } else {
      empty.push({ value: r, label: base })
    }
  }
  return [placeholder, ...empty, ...taken]
})

function apply() {
  emit('apply', buildApplyPayload())
}

/**
 * テスト用に内部状態と操作を露出する。本番 UI は emit / v-model 経由でしか触らないので
 * 副作用は無いが、Vue Test Utils から swap / unassign の単体検証を簡素化できる。
 */
defineExpose({ matches, unmatched, unassignRole, pickRoleFromUnmatched, unassignAll })
</script>

<template>
  <UiModal
    :open="open"
    :title="t('bulkImport.previewTitle')"
    icon="Import"
    size="lg"
    @close="emit('cancel')"
  >
    <div class="bi-source">{{ sourceLabel }} — {{ summaryLine }}</div>

    <div class="bi-protect">
      <button
        type="button"
        class="btn ghost"
        :disabled="matches.length === 0"
        :title="t('bulkImport.unassignAllHint')"
        @click="unassignAll"
      >
        {{ t('bulkImport.unassignAll', { count: matches.length }) }}
      </button>
    </div>

    <div class="bi-columns">
      <section class="bi-col bi-col-unmatched">
        <h4>
          {{
            unmatched.length
              ? t('bulkImport.unmatchedHeader', { count: unmatched.length })
              : t('bulkImport.unmatchedEmpty')
          }}
        </h4>
        <div v-for="u in sortedUnmatched" :key="u.asset.sourcePath" class="bi-unmatched">
          <AniThumb
            v-if="u.asset.ani && u.aniFramesU8"
            :frame-pngs="u.aniFramesU8"
            :sequence="u.asset.ani.sequence"
            :durations="u.asset.ani.perStepDurationsMs"
            :width="64"
            :height="64"
          />
          <img v-else :src="u.previewUrl" :alt="u.asset.sourceFile" />
          <span class="unmatched-meta">{{ u.asset.sourceFile }} ({{ u.asset.width }}px)</span>
          <UiSelect
            :model-value="null"
            width="180px"
            :placeholder="t('bulkImport.selectRolePlaceholder')"
            :options="unmatchedRoleOptions"
            @change="(v) => v && pickRoleFromUnmatched(u, v as string)"
          />
        </div>
        <p v-if="!unmatched.length" class="bi-col-empty">
          {{ t('bulkImport.unmatchedEmptyHint') }}
        </p>
      </section>

      <section class="bi-col bi-col-roles">
        <h4>{{ t('bulkImport.seventeenRoles') }}</h4>
        <BulkImportRoleRow
          v-for="row in allRoleRows"
          :key="row.roleId"
          :role-id="row.roleId"
          :role-label="row.roleLabel"
          :required="row.required"
          :preview-url="row.match?.previewUrl ?? null"
          :source-file="row.match?.asset.sourceFile ?? null"
          :native-size="row.match?.asset.width ?? null"
          :conflict="row.match?.conflict ?? 'none'"
          :ani-data="row.match?.asset.ani ?? null"
          :ani-frames-u8="row.match?.aniFramesU8 ?? null"
          @unassign="unassignRole(row.roleId)"
        />
      </section>
    </div>

    <template v-if="cursorpack">
      <h4>{{ t('bulkImport.metadataHeader') }}</h4>
      <div class="bi-meta-info">
        {{ t('bulkImport.metadataNameLabel') }}: {{ cursorpack.metadata.nameJa ?? '—' }} /
        {{ t('bulkImport.metadataAuthorLabel') }}: {{ cursorpack.metadata.author ?? '—' }} /
        {{ t('bulkImport.metadataVersionLabel') }}: {{ cursorpack.metadata.version ?? '—' }}
      </div>
      <label class="ui-radio" :class="{ 'is-checked': metadataChoice === 'keep' }">
        <input v-model="metadataChoice" type="radio" name="bi-metadata-choice" value="keep" />
        <span class="ui-radio-mark" aria-hidden="true" />
        <span class="ui-radio-label">{{ t('bulkImport.metadataKeep') }}</span>
      </label>
      <label class="ui-radio" :class="{ 'is-checked': metadataChoice === 'overwrite' }">
        <input
          v-model="metadataChoice"
          type="radio"
          name="bi-metadata-choice"
          value="overwrite"
        />
        <span class="ui-radio-mark" aria-hidden="true" />
        <span class="ui-radio-label">{{ t('bulkImport.metadataOverwrite') }}</span>
      </label>
      <label class="ui-radio" :class="{ 'is-checked': metadataChoice === 'name-only' }">
        <input
          v-model="metadataChoice"
          type="radio"
          name="bi-metadata-choice"
          value="name-only"
        />
        <span class="ui-radio-mark" aria-hidden="true" />
        <span class="ui-radio-label">{{ t('bulkImport.metadataNameOnly') }}</span>
      </label>
    </template>

    <template #actions>
      <UiButton variant="ghost" @click="emit('cancel')">{{ t('common.cancel') }}</UiButton>
      <UiButton variant="primary" icon-left="Check" @click="apply">
        {{ t('bulkImport.applyCount', { count: matches.length }) }}
      </UiButton>
    </template>
  </UiModal>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* 2 カラム本体: 狭幅では縦積みにフォールバック (md=768px) */
.bi-columns {
  @apply mt-2 flex flex-col items-start gap-4 md:flex-row;
}
.bi-col {
  @apply min-w-0 flex-1;
}
/* 未マッチカラムは横並び時に sticky 化 — 右カラムをスクロールしても上部に常駐 */
.bi-col-unmatched {
  @apply md:sticky md:top-0 md:max-h-[70vh] md:self-start md:overflow-y-auto;
}
.bi-col-empty {
  @apply mt-2 rounded border border-dashed border-line/60 px-3 py-4 text-center text-[11px] text-fg-dim;
}
.bi-source {
  @apply mb-2 text-[12px] text-fg-mute;
}
.bi-protect {
  @apply mb-3 flex items-center justify-between gap-3 rounded-md border border-line bg-white/[0.02] px-3 py-2;
}
.bi-protect-label {
  @apply text-[12.5px] font-medium text-fg;
}
.bi-unmatched {
  @apply flex items-center gap-3 py-1.5 text-[12px];
}
.bi-unmatched img {
  @apply size-16 object-contain;
  image-rendering: pixelated;
}
.unmatched-meta {
  @apply flex-1 truncate font-mono text-[11px] text-fg-dim;
}
.bi-meta-info {
  @apply mb-1.5 text-[12px] text-fg-mute;
}
h4 {
  @apply my-3 mb-1.5 text-[11px] uppercase tracking-[0.1em] text-fg-mute;
}
</style>
