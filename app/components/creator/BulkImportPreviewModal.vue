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
}

/**
 * 1 ロールに割当て済みのアセット。
 *
 * 確信度パーセントと採用/スキップトグルは廃止。`matches` に入っているもの = apply 対象。
 * 「適用したくない」場合は ✕ ボタンで未マッチプールへ戻すと、apply 時に無視される。
 */
interface PendingMatch {
  role: string
  asset: ResolvedAsset
  conflict: 'none' | 'overwrite-existing' | 'collision-with-other-pending'
  previewUrl: string
  /**
   * `.ani` 用の Uint8Array 化済みフレーム列。Vue 3 のテンプレートは Uint8Array を
   * グローバル名として認識しないため、`new Uint8Array(...)` をテンプレート式で書くと
   * 「Property "Uint8Array" was accessed during render but is not defined on instance」
   * の warn を出す。スクリプト側で 1 度だけ変換しておいて子コンポーネントへ渡す。
   */
  aniFramesU8: readonly Uint8Array[] | null
}

/**
 * まだロールに割当てられていないファイル (未マッチプール)。
 *
 * ドロップダウンでロール選択された瞬間に `matches` へ移動する eager モデルなので、
 * 「選択された未マッチ」という中間状態は持たない。誤マッチで未マッチに戻された
 * ファイルも同じ構造で再利用する。
 */
interface UnmatchedFile {
  asset: ResolvedAsset
  previewUrl: string
  aniFramesU8: readonly Uint8Array[] | null
}

function toAniFramesU8(asset: ResolvedAsset): readonly Uint8Array[] | null {
  return asset.ani ? asset.ani.framePngs.map((b) => new Uint8Array(b)) : null
}

const matches = ref<PendingMatch[]>([])
const unmatched = ref<UnmatchedFile[]>([])
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
          pngBytes: parsed.pngBytes,
          width: parsed.width,
          height: parsed.height,
          hotspot: parsed.hotspot,
          svgText: null,
          availableSizes: isAni ? [parsed.width] : Object.keys(parsed.sizedPngBytes).map(Number),
          ani: parsed.ani,
        }
        const conflict = props.existingRoles.has(role) ? 'overwrite-existing' : 'none'
        matches.value.push({
          role,
          asset: fakeAsset,
          conflict,
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
        candidates.push({ sourceFile: a.sourceFile, nativeSize: a.width, match: m, asset: a })
      } else {
        unmatched.value.push({
          asset: a,
          previewUrl: makePreview(a),
          aniFramesU8: toAniFramesU8(a),
        })
      }
    }
    const { winners, demoted } = resolveCollisions(candidates)
    for (const c of demoted as Array<(typeof candidates)[0]>) {
      unmatched.value.push({
        asset: c.asset,
        previewUrl: makePreview(c.asset),
        aniFramesU8: toAniFramesU8(c.asset),
      })
    }
    for (const w of winners as Array<(typeof candidates)[0]>) {
      const conflict = props.existingRoles.has(w.match.role) ? 'overwrite-existing' : 'none'
      matches.value.push({
        role: w.match.role,
        asset: w.asset,
        conflict,
        previewUrl: makePreview(w.asset),
        aniFramesU8: toAniFramesU8(w.asset),
      })
    }
  },
  { immediate: true },
)

/**
 * 割当解除: matches から該当ロールのファイルを未マッチプールへ戻す。
 *
 * ファジーマッチが誤った場合や、ユーザーが別ロールに付け直したい場合に呼ばれる。
 * previewUrl / aniFramesU8 はそのまま再利用する (revoke しない)。
 */
function unassignRole(roleId: string): void {
  const idx = matches.value.findIndex((m) => m.role === roleId)
  if (idx < 0) return
  const m = matches.value[idx]!
  matches.value.splice(idx, 1)
  unmatched.value.push({
    asset: m.asset,
    previewUrl: m.previewUrl,
    aniFramesU8: m.aniFramesU8,
  })
}

/**
 * 未マッチプールからロールへ割当: 既存の割当があれば未マッチプールへ追い出してから入替。
 *
 * UiSelect の v-model 変更ハンドラから呼ぶ。これにより未マッチドロップダウンで
 * 「すでに別ファイルが入っているロール」を選ぶと、その既存ファイルが未マッチに
 * 戻って入替わる (誤マッチの修正がワンアクションで完了する)。
 */
function pickRoleFromUnmatched(item: UnmatchedFile, roleId: string): void {
  const itemIdx = unmatched.value.indexOf(item)
  if (itemIdx < 0) return

  // 既存割当があれば先に未マッチへ戻す (順序: 自分を抜く → 既存を戻す → 新規割当)
  unmatched.value.splice(itemIdx, 1)
  const existingIdx = matches.value.findIndex((m) => m.role === roleId)
  if (existingIdx >= 0) {
    const existing = matches.value[existingIdx]!
    matches.value.splice(existingIdx, 1)
    unmatched.value.push({
      asset: existing.asset,
      previewUrl: existing.previewUrl,
      aniFramesU8: existing.aniFramesU8,
    })
  }

  const conflict = props.existingRoles.has(roleId) ? 'overwrite-existing' : 'none'
  matches.value.push({
    role: roleId,
    asset: item.asset,
    conflict,
    previewUrl: item.previewUrl,
    aniFramesU8: item.aniFramesU8,
  })
}

/**
 * 全解除: 現在割当済の全ファイルを未マッチプールへ戻す。
 *
 * 「ファジーマッチの結果をまるごとリセットして最初から手動で割り直したい」用途。
 * `.cursorpack` 経路 (全ロールが自動で matches に埋まる) でも、ユーザーが
 * 1 つずつロールを選び直す体験へ切り替えるためのエスケープハッチ。
 */
function unassignAll(): void {
  for (const m of matches.value) {
    unmatched.value.push({
      asset: m.asset,
      previewUrl: m.previewUrl,
      aniFramesU8: m.aniFramesU8,
    })
  }
  matches.value = []
}

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
    asset.kind === 'png' || asset.kind === 'svg' || (asset.hotspot.x === 0 && asset.hotspot.y === 0)
  const finalHotspot =
    isNewRole && noEmbeddedHotspot && !isAni
      ? initialHotspotFor(roleId, asset.width)
      : asset.hotspot
  const base: RoleAsset = {
    primary: new Uint8Array(asset.pngBytes),
    primarySize: asset.width,
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
      const parsed = props.cursorpack.roles[m.role]
      if (!parsed) continue
      // SizedAsset 形式で構築 (per-size hotspot は cursorpack 内では未サポートなので undefined)
      const sized = new Map<number, import('~/composables/useCreatorAssets').SizedAsset>()
      for (const [k, v] of Object.entries(parsed.sizedPngBytes)) {
        sized.set(Number(k), { png: new Uint8Array(v) })
      }
      const asset: RoleAsset = {
        primary: new Uint8Array(parsed.pngBytes),
        primarySize: parsed.width,
        hotspot: parsed.hotspot,
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
    // unmatched は eager に matches へ移動するモデルなので、apply 時点では
    // ロール確定済みの matches だけを走査すればよい。未マッチに残ったまま
    // apply された場合はそのファイルは破棄される (UI 側でも未マッチ件数として可視化)。
    const sourceTag: RoleAsset['source'] =
      (props.resolved?.length ?? 0) > 1 ? 'bulk-folder' : 'bulk-file'
    for (const m of matches.value) {
      roleAssets.push({
        roleId: m.role,
        asset: toRoleAsset(m.role, m.asset, sourceTag, !props.existingRoles.has(m.role)),
      })
    }
  }

  emit('apply', {
    roleAssets,
    metadataChoice: metadataChoice.value,
    metadata: props.cursorpack?.metadata ?? null,
  })
}

onUnmounted(resetState)

/**
 * テスト用に内部状態と操作を露出する。本番 UI は emit / v-model 経由でしか触らないので
 * 副作用は無いが、Vue Test Utils から swap / unassign の単体検証を簡素化できる。
 */
defineExpose({ matches, unmatched, unassignRole, pickRoleFromUnmatched, unassignAll })
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
      </div>

      <footer class="bi-foot">
        <button class="btn ghost ml-auto" @click="emit('cancel')">{{ t('common.cancel') }}</button>
        <button class="btn primary" @click="apply">
          ✓
          {{ t('bulkImport.applyCount', { count: matches.length }) }}
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
  @apply flex max-h-[90vh] w-[min(1200px,96vw)] flex-col rounded-[12px] border border-line;
  background: var(--bg-1, #14161c);
}
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
