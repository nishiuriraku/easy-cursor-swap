<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import {
  CURSOR_ROLE_IDS,
  matchAssetToRole,
  resolveCollisions,
  type MatchCandidate,
} from '~/composables/useRoleMatcher'
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import type { ResolvedAsset, ParsedCursorpack } from '~/composables/useBulkImport'
import type { RoleAsset } from '~/composables/useCreatorAssets'
import BulkImportRoleRow from './BulkImportRoleRow.vue'

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
  roleAssets: Array<{ roleId: string, asset: RoleAsset }>
  metadataChoice: 'keep' | 'overwrite' | 'name-only'
  metadata: ParsedCursorpack['metadata'] | null
}

interface PendingMatch {
  role: string
  asset: ResolvedAsset
  confidence: number
  conflict: 'none' | 'overwrite-existing' | 'collision-with-other-pending'
  decision: 'apply' | 'skip'
  previewUrl: string
}

interface UnmatchedFile {
  asset: ResolvedAsset
  manuallyAssignedRole: string | null
  previewUrl: string
}

const protectExisting = ref(true)
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

watch(() => props.open, (open) => {
  if (!open) { resetState(); return }
  resetState()

  if (props.cursorpack) {
    // .cursorpack 経路: ロール ID は既に確定済み。全ロールを matches に詰める。
    for (const [role, parsed] of Object.entries(props.cursorpack.roles)) {
      const fakeAsset: ResolvedAsset = {
        sourceFile: `${role}.cur`,
        sourcePath: '',
        kind: 'cur',
        pngBytes: parsed.primaryPngBytes,
        svgText: null,
        nativeSize: parsed.primarySize,
        hotspotX: parsed.hotspotX,
        hotspotY: parsed.hotspotY,
        availableSizes: Object.keys(parsed.sizedPngBytes).map(Number),
      }
      const conflict = props.existingRoles.has(role) ? 'overwrite-existing' : 'none'
      matches.value.push({
        role,
        asset: fakeAsset,
        confidence: 1.0,
        conflict,
        decision: conflict === 'overwrite-existing' && protectExisting.value ? 'skip' : 'apply',
        previewUrl: makePreview(fakeAsset),
      })
    }
    return
  }

  // 通常経路: ファジーマッチ → 衝突解決 → 既存衝突判定
  const candidates: Array<MatchCandidate & { asset: ResolvedAsset }> = []
  for (const a of props.resolved ?? []) {
    const m = matchAssetToRole(a.sourceFile)
    if (m) {
      candidates.push({ sourceFile: a.sourceFile, nativeSize: a.nativeSize, match: m, asset: a })
    } else {
      unmatched.value.push({ asset: a, manuallyAssignedRole: null, previewUrl: makePreview(a) })
    }
  }
  const { winners, demoted } = resolveCollisions(candidates)
  for (const c of demoted as Array<typeof candidates[0]>) {
    unmatched.value.push({ asset: c.asset, manuallyAssignedRole: null, previewUrl: makePreview(c.asset) })
  }
  for (const w of winners as Array<typeof candidates[0]>) {
    const conflict = props.existingRoles.has(w.match.role) ? 'overwrite-existing' : 'none'
    matches.value.push({
      role: w.match.role,
      asset: w.asset,
      confidence: w.match.score,
      conflict,
      decision: conflict === 'overwrite-existing' && protectExisting.value ? 'skip' : 'apply',
      previewUrl: makePreview(w.asset),
    })
  }
}, { immediate: true })

watch(protectExisting, (v) => {
  for (const m of matches.value) {
    if (m.conflict === 'overwrite-existing') {
      m.decision = v ? 'skip' : 'apply'
    }
  }
})

const summaryLine = computed(() => {
  const auto = matches.value.filter(m => m.decision === 'apply').length
  const conflicts = matches.value.filter(m => m.conflict !== 'none').length
  return `自動 ${auto} 件 ・ 未マッチ ${unmatched.value.length} 件 ・ 衝突 ${conflicts} 件`
})

const allRoleRows = computed(() => {
  const byRole = new Map(matches.value.map(m => [m.role, m]))
  return CURSOR_ROLE_IDS.map(roleId => {
    const m = byRole.get(roleId)
    const def = CURSOR_ROLES.find(r => r.id === roleId)
    return {
      roleId,
      roleLabel: def?.jp ?? roleId,
      required: roleId === 'Arrow',
      match: m,
    }
  })
})

function toRoleAsset(asset: ResolvedAsset, source: RoleAsset['source']): RoleAsset {
  return {
    primary: new Uint8Array(asset.pngBytes),
    primarySize: asset.nativeSize,
    hotspot: { x: asset.hotspotX, y: asset.hotspotY },
    sized: undefined,
    source,
  }
}

function apply() {
  const roleAssets: Array<{ roleId: string, asset: RoleAsset }> = []

  if (props.cursorpack) {
    for (const m of matches.value) {
      if (m.decision !== 'apply') continue
      const parsed = props.cursorpack.roles[m.role]
      const sized = new Map<number, Uint8Array>()
      for (const [k, v] of Object.entries(parsed.sizedPngBytes)) {
        sized.set(Number(k), new Uint8Array(v))
      }
      roleAssets.push({
        roleId: m.role,
        asset: {
          primary: new Uint8Array(parsed.primaryPngBytes),
          primarySize: parsed.primarySize,
          hotspot: { x: parsed.hotspotX, y: parsed.hotspotY },
          sized,
          source: 'cursorpack',
        },
      })
    }
  } else {
    const sourceTag: RoleAsset['source'] =
      (props.resolved?.length ?? 0) > 1 ? 'bulk-folder' : 'bulk-file'
    for (const m of matches.value) {
      if (m.decision !== 'apply') continue
      roleAssets.push({ roleId: m.role, asset: toRoleAsset(m.asset, sourceTag) })
    }
    for (const u of unmatched.value) {
      if (!u.manuallyAssignedRole) continue
      roleAssets.push({ roleId: u.manuallyAssignedRole, asset: toRoleAsset(u.asset, sourceTag) })
    }
  }

  emit('apply', {
    roleAssets,
    metadataChoice: metadataChoice.value,
    metadata: props.cursorpack?.metadata ?? null,
  })
}

onUnmounted(resetState)
</script>

<template>
  <div v-if="open" class="bi-overlay" @click.self="emit('cancel')">
    <div class="bi-modal" role="dialog" aria-modal="true">
      <header class="bi-head">
        <h3>一括インポート プレビュー</h3>
        <button class="btn ghost" @click="emit('cancel')">✕</button>
      </header>
      <div class="bi-body">
        <div class="bi-source">{{ sourceLabel }} — {{ summaryLine }}</div>

        <label class="bi-protect">
          <input v-model="protectExisting" type="checkbox" />
          既存ロールを保護する
        </label>

        <h4>17 ロール</h4>
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
          @toggle="(v) => row.match && (row.match.decision = v)"
        />

        <h4 v-if="unmatched.length">未マッチファイル ({{ unmatched.length }})</h4>
        <div v-for="u in unmatched" :key="u.asset.sourcePath" class="bi-unmatched">
          <img :src="u.previewUrl" :alt="u.asset.sourceFile" />
          <span>{{ u.asset.sourceFile }} ({{ u.asset.nativeSize }}px)</span>
          <select v-model="u.manuallyAssignedRole">
            <option :value="null">— ロール選択 —</option>
            <option v-for="r in CURSOR_ROLE_IDS" :key="r" :value="r">{{ r }}</option>
          </select>
        </div>

        <template v-if="cursorpack">
          <h4>パッケージ メタデータ</h4>
          <div class="bi-meta-info">
            名前: {{ cursorpack.metadata.nameJa ?? '—' }} /
            作者: {{ cursorpack.metadata.author ?? '—' }} /
            Version: {{ cursorpack.metadata.version ?? '—' }}
          </div>
          <label><input v-model="metadataChoice" type="radio" value="keep" /> 現在の編集を保持</label>
          <label><input v-model="metadataChoice" type="radio" value="overwrite" /> 取り込んだメタデータで上書き</label>
          <label><input v-model="metadataChoice" type="radio" value="name-only" /> 名前のみ採用</label>
        </template>
      </div>

      <footer class="bi-foot">
        <button class="btn ghost" @click="emit('cancel')">キャンセル</button>
        <button class="btn primary" @click="apply">
          ✓ {{ matches.filter(m => m.decision === 'apply').length
              + unmatched.filter(u => u.manuallyAssignedRole !== null).length }} 件を適用
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.bi-overlay {
  position: fixed; inset: 0;
  background: rgba(10, 11, 15, 0.7);
  display: flex; align-items: center; justify-content: center;
  z-index: 100;
}
.bi-modal {
  background: var(--bg-1, #14161c);
  border: 1px solid var(--line);
  border-radius: 12px;
  width: min(900px, 96vw);
  max-height: 90vh;
  display: flex; flex-direction: column;
}
.bi-head, .bi-foot {
  padding: 12px 18px;
  border-bottom: 1px solid var(--line);
  display: flex; justify-content: space-between; align-items: center;
}
.bi-foot { border-bottom: 0; border-top: 1px solid var(--line); gap: 8px; justify-content: flex-end; }
.bi-body { padding: 12px 18px; overflow-y: auto; flex: 1; }
.bi-source { font-size: 12px; color: var(--fg-mute); margin-bottom: 8px; }
.bi-protect { display: inline-flex; gap: 6px; margin-bottom: 12px; font-size: 12px; }
.bi-unmatched { display: flex; align-items: center; gap: 8px; padding: 4px 0; font-size: 12px; }
.bi-unmatched img { width: 32px; height: 32px; object-fit: contain; }
.bi-meta-info { font-size: 12px; color: var(--fg-mute); margin-bottom: 6px; }
h4 { font-size: 11px; color: var(--fg-mute); margin: 12px 0 6px; letter-spacing: 0.1em; text-transform: uppercase; }
</style>
