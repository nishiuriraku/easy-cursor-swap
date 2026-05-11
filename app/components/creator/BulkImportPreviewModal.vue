<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import {
  CURSOR_ROLE_IDS,
  matchAssetWithContext,
  resolveCollisions,
  type MatchCandidate,
} from '~/composables/useRoleMatcher'
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import type { ResolvedAsset, ParsedCursorpack } from '~/composables/useBulkImport'
import type { RoleAsset } from '~/composables/useCreatorAssets'
import { initialHotspotFor } from '~/composables/useHotspotDefaults'
import { useI18n } from '~/composables/useI18n'
import BulkImportRoleRow from './BulkImportRoleRow.vue'
import AniThumb from './AniThumb.vue'

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

export interface ApplyPayload {
  /** 確定したロール → アセット。useCreatorAssets.setAsset() で書き込む。 */
  roleAssets: Array<{ roleId: string; asset: RoleAsset }>
  metadataChoice: 'keep' | 'overwrite' | 'name-only'
  metadata: ParsedCursorpack['metadata'] | null
  /** ✓ なら親側で SaveDestinationModal を libraryAndApply で自動オープン */
  applyImmediately: boolean
}

interface PendingMatch {
  role: string
  asset: ResolvedAsset
  confidence: number
  conflict: 'none' | 'overwrite-existing' | 'collision-with-other-pending'
  decision: 'apply' | 'skip'
  previewUrl: string
  /**
   * `.ani` 用の Uint8Array 化済みフレーム列。Vue 3 のテンプレートは Uint8Array を
   * グローバル名として認識しないため、`new Uint8Array(...)` をテンプレート式で書くと
   * 「Property "Uint8Array" was accessed during render but is not defined on instance」
   * の warn を出す。スクリプト側で 1 度だけ変換しておいて子コンポーネントへ渡す。
   */
  aniFramesU8: readonly Uint8Array[] | null
}

interface UnmatchedFile {
  asset: ResolvedAsset
  manuallyAssignedRole: string | null
  previewUrl: string
  aniFramesU8: readonly Uint8Array[] | null
}

function toAniFramesU8(asset: ResolvedAsset): readonly Uint8Array[] | null {
  return asset.ani ? asset.ani.framePngs.map((b) => new Uint8Array(b)) : null
}

const protectExisting = ref(true)
const applyImmediately = ref(false)
const matches = ref<PendingMatch[]>([])
const unmatched = ref<UnmatchedFile[]>([])
const skippedCount = ref(0)
const metadataChoice = ref<'keep' | 'overwrite' | 'name-only'>('overwrite')

const previewUrls: string[] = []

function makePreview(asset: ResolvedAsset): string {
  if (asset.kind === 'svg' && asset.svgText) {
    const url = URL.createObjectURL(new Blob([asset.svgText], { type: 'image/svg+xml' }))
    previewUrls.push(url)
    return url
  }
  const url = URL.createObjectURL(new Blob([new Uint8Array(asset.pngBytes)], { type: 'image/png' }))
  previewUrls.push(url)
  return url
}

function resetState() {
  for (const u of previewUrls) URL.revokeObjectURL(u)
  previewUrls.length = 0
  matches.value = []
  unmatched.value = []
  skippedCount.value = 0
}

watch(
  () => props.open,
  (open) => {
    if (!open) {
      resetState()
      return
    }
    resetState()

    if (props.cursorpack) {
      // .cursorpack 経路: ロール ID は既に確定済み。全ロールを matches に詰める。
      for (const [role, parsed] of Object.entries(props.cursorpack.roles)) {
        const isAni = parsed.ani !== null
        const fakeAsset: ResolvedAsset = {
          sourceFile: isAni ? `${role}.ani` : `${role}.cur`,
          // `.ani` の場合は展開先絶対パスを sourcePath に入れる
          // (export 時に Rust が rewrite_ani_with_hotspot のソースとして使う)
          sourcePath: parsed.aniSourcePath ?? '',
          kind: isAni ? 'ani' : 'cur',
          pngBytes: parsed.primaryPngBytes,
          svgText: null,
          nativeSize: parsed.primarySize,
          hotspotX: parsed.hotspotX,
          hotspotY: parsed.hotspotY,
          availableSizes: isAni
            ? [parsed.primarySize]
            : Object.keys(parsed.sizedPngBytes).map(Number),
          ani: parsed.ani,
        }
        const conflict = props.existingRoles.has(role) ? 'overwrite-existing' : 'none'
        matches.value.push({
          role,
          asset: fakeAsset,
          confidence: 1.0,
          conflict,
          decision: conflict === 'overwrite-existing' && protectExisting.value ? 'skip' : 'apply',
          previewUrl: makePreview(fakeAsset),
          aniFramesU8: toAniFramesU8(fakeAsset),
        })
      }
      return
    }

    // 通常経路: ファイル名 + フォルダ名のファジーマッチ → 衝突解決 → 既存衝突判定。
    // sourcePath を渡すことで `arrow/64.png` `通常/256.png` のように
    // フォルダー名にロール名が含まれるケースもマッチさせる。
    const candidates: Array<MatchCandidate & { asset: ResolvedAsset }> = []
    for (const a of props.resolved ?? []) {
      const m = matchAssetWithContext(a.sourceFile, a.sourcePath)
      if (m) {
        candidates.push({ sourceFile: a.sourceFile, nativeSize: a.nativeSize, match: m, asset: a })
      } else {
        unmatched.value.push({
          asset: a,
          manuallyAssignedRole: null,
          previewUrl: makePreview(a),
          aniFramesU8: toAniFramesU8(a),
        })
      }
    }
    const { winners, demoted } = resolveCollisions(candidates)
    for (const c of demoted as Array<(typeof candidates)[0]>) {
      unmatched.value.push({
        asset: c.asset,
        manuallyAssignedRole: null,
        previewUrl: makePreview(c.asset),
        aniFramesU8: toAniFramesU8(c.asset),
      })
    }
    for (const w of winners as Array<(typeof candidates)[0]>) {
      const conflict = props.existingRoles.has(w.match.role) ? 'overwrite-existing' : 'none'
      matches.value.push({
        role: w.match.role,
        asset: w.asset,
        confidence: w.match.score,
        conflict,
        decision: conflict === 'overwrite-existing' && protectExisting.value ? 'skip' : 'apply',
        previewUrl: makePreview(w.asset),
        aniFramesU8: toAniFramesU8(w.asset),
      })
    }
  },
  { immediate: true },
)

watch(protectExisting, (v) => {
  for (const m of matches.value) {
    if (m.conflict === 'overwrite-existing') {
      m.decision = v ? 'skip' : 'apply'
    }
  }
})

const summaryLine = computed(() => {
  const auto = matches.value.filter((m) => m.decision === 'apply').length
  const conflicts = matches.value.filter((m) => m.conflict !== 'none').length
  return `自動 ${auto} 件 ・ 未マッチ ${unmatched.value.length} 件 ・ 衝突 ${conflicts} 件`
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

function toRoleAsset(
  roleId: string,
  asset: ResolvedAsset,
  source: RoleAsset['source'],
  isNewRole: boolean,
): RoleAsset {
  // 「元ファイルに hotspot 情報がない」を kind で判定する。
  // PNG / SVG は仕様上 hotspot を持たない (Rust 側 bulk_resolve は (0,0) を返す)。
  // CUR / ICO は header に hotspot があるが、未指定の場合 (0,0) で来る — そのときも
  // 中央既定が望ましい。`(4, 4)` で判定すると Rust の sentinel (= 0) と噛み合わず
  // 中央化が一切発火しないバグになる。
  // .ani は自前の埋め込みホットスポットを使う (中央既定を適用しない)。
  const isAni = asset.kind === 'ani' && asset.ani !== null
  const noEmbeddedHotspot =
    asset.kind === 'png' || asset.kind === 'svg' || (asset.hotspotX === 0 && asset.hotspotY === 0)
  const finalHotspot =
    isNewRole && noEmbeddedHotspot && !isAni
      ? initialHotspotFor(roleId, asset.nativeSize)
      : { x: asset.hotspotX, y: asset.hotspotY }
  const base: RoleAsset = {
    primary: new Uint8Array(asset.pngBytes),
    primarySize: asset.nativeSize,
    hotspot: finalHotspot,
    sized: undefined,
    source,
  }
  if (isAni && asset.ani) {
    base.aniSourcePath = asset.sourcePath
    base.aniFrames = {
      framePngs: asset.ani.framePngs.map((b) => new Uint8Array(b)),
      sequence: asset.ani.sequence,
      perStepDurationsMs: asset.ani.perStepDurationsMs,
    }
  }
  return base
}

function apply() {
  const roleAssets: Array<{ roleId: string; asset: RoleAsset }> = []

  if (props.cursorpack) {
    for (const m of matches.value) {
      if (m.decision !== 'apply') continue
      const parsed = props.cursorpack.roles[m.role]
      const sized = new Map<number, Uint8Array>()
      for (const [k, v] of Object.entries(parsed.sizedPngBytes)) {
        sized.set(Number(k), new Uint8Array(v))
      }
      const asset: RoleAsset = {
        primary: new Uint8Array(parsed.primaryPngBytes),
        primarySize: parsed.primarySize,
        hotspot: { x: parsed.hotspotX, y: parsed.hotspotY },
        sized: sized.size > 0 ? sized : undefined,
        source: 'cursorpack',
      }
      // `.ani` ロールはアニメーション情報を保持して export 時に動的カーソルとして再構築する
      if (parsed.ani && parsed.aniSourcePath) {
        asset.aniSourcePath = parsed.aniSourcePath
        asset.aniFrames = {
          framePngs: parsed.ani.framePngs.map((b) => new Uint8Array(b)),
          sequence: parsed.ani.sequence,
          perStepDurationsMs: parsed.ani.perStepDurationsMs,
        }
      }
      roleAssets.push({ roleId: m.role, asset })
    }
  } else {
    const sourceTag: RoleAsset['source'] =
      (props.resolved?.length ?? 0) > 1 ? 'bulk-folder' : 'bulk-file'
    for (const m of matches.value) {
      if (m.decision !== 'apply') continue
      roleAssets.push({
        roleId: m.role,
        asset: toRoleAsset(m.role, m.asset, sourceTag, !props.existingRoles.has(m.role)),
      })
    }
    for (const u of unmatched.value) {
      if (!u.manuallyAssignedRole) continue
      roleAssets.push({
        roleId: u.manuallyAssignedRole,
        asset: toRoleAsset(
          u.manuallyAssignedRole,
          u.asset,
          sourceTag,
          !props.existingRoles.has(u.manuallyAssignedRole),
        ),
      })
    }
  }

  emit('apply', {
    roleAssets,
    metadataChoice: metadataChoice.value,
    metadata: props.cursorpack?.metadata ?? null,
    applyImmediately: applyImmediately.value,
  })
}

onUnmounted(resetState)
</script>

<template>
  <div v-if="open" class="bi-overlay" @click.self="emit('cancel')">
    <div class="bi-modal" role="dialog" aria-modal="true">
      <header class="bi-head">
        <h3>{{ t('bulkImport.previewTitle') }}</h3>
        <button class="btn ghost" :aria-label="t('common.close')" @click="emit('cancel')">✕</button>
      </header>
      <div class="bi-body">
        <div class="bi-source">{{ sourceLabel }} — {{ summaryLine }}</div>

        <label class="bi-protect">
          <input v-model="protectExisting" type="checkbox" />
          {{ t('bulkImport.protectExisting') }}
        </label>

        <h4>{{ t('bulkImport.seventeenRoles') }}</h4>
        <BulkImportRoleRow
          v-for="row in allRoleRows"
          :key="row.roleId"
          :role-id="row.roleId"
          :role-label="row.roleLabel"
          :required="row.required"
          :preview-url="row.match?.previewUrl ?? null"
          :source-file="row.match?.asset.sourceFile ?? null"
          :native-size="row.match?.asset.nativeSize ?? null"
          :confidence="row.match?.confidence ?? null"
          :conflict="row.match?.conflict ?? 'none'"
          :decision="row.match?.decision ?? 'skip'"
          :ani-data="row.match?.asset.ani ?? null"
          :ani-frames-u8="row.match?.aniFramesU8 ?? null"
          @toggle="(v) => row.match && (row.match.decision = v)"
        />

        <h4 v-if="unmatched.length">
          {{ t('bulkImport.unmatchedHeader', { count: unmatched.length }) }}
        </h4>
        <div v-for="u in unmatched" :key="u.asset.sourcePath" class="bi-unmatched">
          <AniThumb
            v-if="u.asset.ani && u.aniFramesU8"
            :frame-pngs="u.aniFramesU8"
            :sequence="u.asset.ani.sequence"
            :durations="u.asset.ani.perStepDurationsMs"
            :width="32"
            :height="32"
          />
          <img v-else :src="u.previewUrl" :alt="u.asset.sourceFile" />
          <span>{{ u.asset.sourceFile }} ({{ u.asset.nativeSize }}px)</span>
          <UiSelect
            v-model="u.manuallyAssignedRole"
            width="180px"
            :placeholder="t('bulkImport.selectRolePlaceholder')"
            :options="[
              { value: null, label: t('bulkImport.selectRolePlaceholder') },
              ...CURSOR_ROLE_IDS.map((r) => {
                const def = CURSOR_ROLES.find((d) => d.id === r)
                return { value: r, label: def ? `${def.jp}（${r}）` : r }
              }),
            ]"
          />
        </div>

        <template v-if="cursorpack">
          <h4>{{ t('bulkImport.metadataHeader') }}</h4>
          <div class="bi-meta-info">
            {{ t('bulkImport.metadataNameLabel') }}: {{ cursorpack.metadata.nameJa ?? '—' }} /
            {{ t('bulkImport.metadataAuthorLabel') }}: {{ cursorpack.metadata.author ?? '—' }} /
            {{ t('bulkImport.metadataVersionLabel') }}: {{ cursorpack.metadata.version ?? '—' }}
          </div>
          <label
            ><input v-model="metadataChoice" type="radio" value="keep" />
            {{ t('bulkImport.metadataKeep') }}</label
          >
          <label
            ><input v-model="metadataChoice" type="radio" value="overwrite" />
            {{ t('bulkImport.metadataOverwrite') }}</label
          >
          <label
            ><input v-model="metadataChoice" type="radio" value="name-only" />
            {{ t('bulkImport.metadataNameOnly') }}</label
          >
        </template>
      </div>

      <footer class="bi-foot">
        <label class="bi-apply-immediately">
          <input v-model="applyImmediately" type="checkbox" data-test="apply-immediately" />
          {{ t('bulkImport.applyImmediately') }}
        </label>
        <button class="btn ghost ml-auto" @click="emit('cancel')">{{ t('common.cancel') }}</button>
        <button class="btn primary" @click="apply">
          ✓
          {{
            t('bulkImport.applyCount', {
              count:
                matches.filter((m) => m.decision === 'apply').length +
                unmatched.filter((u) => u.manuallyAssignedRole !== null).length,
            })
          }}
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.bi-overlay {
  @apply fixed inset-0 z-[100] flex items-center justify-center bg-[rgba(10,11,15,0.7)];
}
.bi-modal {
  @apply flex max-h-[90vh] w-[min(900px,96vw)] flex-col rounded-[12px] border border-line;
  background: var(--bg-1, #14161c);
}
.bi-head,
.bi-foot {
  @apply flex items-center justify-between border-b border-line px-4 py-3;
}
.bi-foot {
  @apply justify-end gap-2 border-b-0 border-t border-line;
}
.bi-body {
  @apply flex-1 overflow-y-auto px-4 py-3;
}
.bi-source {
  @apply mb-2 text-[12px] text-fg-mute;
}
.bi-protect {
  @apply mb-3 inline-flex gap-1.5 text-[12px];
}
.bi-apply-immediately {
  @apply inline-flex items-center gap-1.5 text-[12px];
}
.bi-unmatched {
  @apply flex items-center gap-2 py-1 text-[12px];
}
.bi-unmatched img {
  @apply size-8 object-contain;
}
.bi-meta-info {
  @apply mb-1.5 text-[12px] text-fg-mute;
}
h4 {
  @apply my-3 mb-1.5 text-[11px] uppercase tracking-[0.1em] text-fg-mute;
}
</style>
