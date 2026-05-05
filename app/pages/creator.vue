<script setup lang="ts">
/**
 * クリエイターモード (Phase 5-5)
 *
 * design/creator.jsx を Vue 化したもの。3 カラム構成:
 *  - 左:  17 役割リスト (filled/partial/empty ドット付き)
 *  - 中央: ビッグプレビュー + 6 サイズストリップ + リサンプル切替
 *  - 右:  Hotspot / アセット / Validation の各プロパティ
 *
 * NOTE: 実際の画像アップロード / .cur ビルド / 署名生成は今回はスタブ。
 *       UI 構造とインタラクションのみ実装し、IPC 配線は後続タスクに委ねる。
 */
import { computed, ref } from 'vue'
import { CURSOR_ROLES, type CursorRoleDef } from '~/components/icons/CursorIcons'
import { sanitizeSvg } from '~/composables/sanitizeSvg'
import { invokeTauri } from '~/composables/useTauri'
import { useKeystore } from '~/composables/useKeystore'

const { info: keystoreInfo, refresh: refreshKeystore } = useKeystore()
const hasKeystoreSigning = computed(() => keystoreInfo.value.has_keypair)

type RoleStatus = 'filled' | 'partial' | 'empty'
type ResampleMode = 'lanczos' | 'nearest' | 'auto'

const SIZES = [32, 48, 64, 96, 128, 256] as const
type TabId = 'assign' | 'metadata' | 'preview' | 'publish'

// --- ダミーステート (実装は将来の IPC 連携で置換) ---
const filledRoles = new Set<string>([
  'Arrow', 'Help', 'Wait', 'IBeam', 'Hand', 'No', 'Crosshair',
  'SizeNS', 'SizeWE', 'SizeAll', 'NWPen',
])
const partialRoles = new Set<string>(['AppStarting', 'SizeNWSE'])

const activeTab = ref<TabId>('assign')
const activeRoleId = ref<string>('Arrow')
const activeSize = ref<number>(64)
const filledSizesByRole = ref<Record<string, number[]>>({
  Arrow: [32, 48, 64, 128],
})
const resample = ref<ResampleMode>('lanczos')
const hotspotX = ref(4)
const hotspotY = ref(4)
const perSizeHotspot = ref(true)
const shadowEnabled = ref(false)

/**
 * 役割ごとのインポート済み PNG バイト + 役割ごとに保存済みの一時 .cur パス。
 * `assignedPng[role]` に画像があれば、エクスポート時に Rust 側で .cur を生成する。
 */
const assignedPng = ref<Record<string, Uint8Array>>({})
const assignedHotspot = ref<Record<string, { x: number, y: number }>>({})

// メタデータタブの入力
const metaName = ref<string>('Untitled Theme')
const metaNameEn = ref<string>('')
const metaAuthor = ref<string>('')
const metaVersion = ref<string>('1.0.0')
const metaDescription = ref<string>('')

// --- 計算プロパティ ---
const activeRole = computed<CursorRoleDef>(() =>
  CURSOR_ROLES.find((r) => r.id === activeRoleId.value) ?? CURSOR_ROLES[0]!,
)

function statusOf(id: string): RoleStatus {
  if (filledRoles.has(id)) return 'filled'
  if (partialRoles.has(id)) return 'partial'
  return 'empty'
}

const filledCount = computed(() => filledRoles.size)
const tabs = computed<Array<{ id: TabId, label: string, count?: string }>>(() => [
  { id: 'assign', label: '割り当て', count: `${filledCount.value}/17` },
  { id: 'metadata', label: 'メタデータ' },
  { id: 'preview', label: 'プレビュー' },
  { id: 'publish', label: '公開' },
])
const filledSizes = computed(
  () => filledSizesByRole.value[activeRoleId.value] ?? [],
)
const sizesCovered = computed(() => filledSizes.value.length)

function selectRole(id: string) {
  activeRoleId.value = id
}

function selectSize(s: number) {
  activeSize.value = s
}

function isRequired(id: string): boolean {
  return id === 'Arrow'
}

// 起動時に keystore 状態を取得して「署名 & エクスポート」ボタンの表示判定に使う
import { onMounted } from 'vue'
onMounted(() => {
  void refreshKeystore()
})

// --- 画像インポート ---
const importBusy = ref(false)
const importMessage = ref<string | null>(null)
/** 直近インポートで除去された SVG 要素/属性 (デバッグ表示) */
const sanitizedRemovals = ref<string[]>([])
/** プレビュー用 URL (Object URL or data URL) */
const importedPreviewUrl = ref<string | null>(null)
/** ビルドに送る PNG バイト列。SVG インポート時は Canvas 経由で PNG 化したものを保持。 */
const importedPngBytes = ref<Uint8Array | null>(null)

const fileInput = ref<HTMLInputElement | null>(null)

function pickImage() {
  fileInput.value?.click()
}

// --- ビルド & パッケージエクスポート ---
const buildBusy = ref(false)
const buildMessage = ref<string | null>(null)
const exportBusy = ref(false)
const exportMessage = ref<string | null>(null)

const assignedRoleCount = computed(() => Object.keys(assignedPng.value).length)
const arrowAssigned = computed(() => 'Arrow' in assignedPng.value)

interface ExportResult {
  theme_id: string
  size_bytes: number
  signed: boolean
  key_id: string | null
}

/** クリエイターの全状態を `.cursorpack` として書き出す。 */
async function exportCursorpack(opts: { sign: boolean }) {
  if (assignedRoleCount.value === 0) {
    exportMessage.value = '少なくとも 1 役割に画像を割り当ててください'
    return
  }
  if (!arrowAssigned.value) {
    exportMessage.value = 'Arrow ロールは必須です'
    return
  }
  exportBusy.value = true
  exportMessage.value = null

  // 一時ディレクトリに各役割の .cur を吐き出す
  // (現状は appDataDir/temp/ に書く運用。将来は Rust 側でメモリ上完結に置換)
  const tempCurPaths: Record<string, string> = {}
  let tempBase: string | null = null
  try {
    const { tempDir, join } = await import('@tauri-apps/api/path')
    tempBase = await tempDir()
    const sessionDir = await join(tempBase, `cursor-forge-${Date.now()}`)

    for (const [role, png] of Object.entries(assignedPng.value)) {
      const hot = assignedHotspot.value[role] ?? { x: 0, y: 0 }
      const target = await join(sessionDir, `${role}.cur`)
      await invokeTauri<number>('build_cursor_file', {
        req: {
          pngBytes: Array.from(png),
          hotspotX: hot.x,
          hotspotY: hot.y,
          resample: resample.value,
          outputPath: target,
        },
      })
      tempCurPaths[role] = target
    }

    const { save } = await import('@tauri-apps/plugin-dialog')
    const safeName = metaName.value.replace(/[^\p{L}\p{N}_-]+/gu, '_').slice(0, 64) || 'theme'
    const target = await save({
      defaultPath: `${safeName}.cursorpack`,
      filters: [{ name: 'Cursor Pack', extensions: ['cursorpack'] }],
    })
    if (!target) {
      exportMessage.value = '保存先が選択されませんでした'
      return
    }

    const result = await invokeTauri<ExportResult>('export_cursorpack', {
      req: {
        nameJa: metaName.value,
        nameEn: metaNameEn.value || null,
        author: metaAuthor.value || null,
        version: metaVersion.value,
        requiresOsShadow: shadowEnabled.value,
        hotspots: assignedHotspot.value,
        curPaths: tempCurPaths,
        outputPath: target,
        sign: opts.sign,
      },
    })

    if (!result) throw new Error('エクスポート結果が空でした')
    const sigText = result.signed ? `署名: ${result.key_id ?? '?'}` : '未署名'
    exportMessage.value = `.cursorpack を書き出しました (${result.size_bytes} bytes, ${sigText}) → ${target}`
  } catch (err) {
    exportMessage.value = `エクスポート失敗: ${err instanceof Error ? err.message : String(err)}`
  } finally {
    exportBusy.value = false
  }
}

async function buildAndSave() {
  if (!importedPngBytes.value) {
    buildMessage.value = 'まず PNG/SVG 画像を読み込んでください'
    return
  }
  buildBusy.value = true
  buildMessage.value = null
  try {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const target = await save({
      defaultPath: `${activeRole.value.id}.cur`,
      filters: [{ name: 'Cursor', extensions: ['cur'] }],
    })
    if (!target) return

    const written = await invokeTauri<number>('build_cursor_file', {
      req: {
        pngBytes: Array.from(importedPngBytes.value),
        hotspotX: hotspotX.value,
        hotspotY: hotspotY.value,
        resample: resample.value,
        outputPath: target,
      },
    })
    buildMessage.value = `.cur をビルドしました (${written ?? '?'} bytes) → ${target}`
  } catch (err) {
    buildMessage.value = `ビルド失敗: ${err instanceof Error ? err.message : String(err)}`
  } finally {
    buildBusy.value = false
  }
}

/** sanitized SVG 文字列 → 指定サイズの PNG バイト列 (Canvas 経由) */
async function rasterizeSvgToPng(svgString: string, size: number): Promise<Uint8Array> {
  const blob = new Blob([svgString], { type: 'image/svg+xml' })
  const url = URL.createObjectURL(blob)
  try {
    const img = new Image()
    img.decoding = 'async'
    img.src = url
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve()
      img.onerror = () => reject(new Error('SVG イメージの読み込みに失敗'))
    })
    const canvas = document.createElement('canvas')
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext('2d')
    if (!ctx) throw new Error('Canvas 2D コンテキスト取得失敗')
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(img, 0, 0, size, size)
    const pngBlob: Blob = await new Promise((resolve, reject) => {
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('toBlob 失敗'))), 'image/png')
    })
    return new Uint8Array(await pngBlob.arrayBuffer())
  } finally {
    URL.revokeObjectURL(url)
  }
}

async function onFileChange(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  importBusy.value = true
  importMessage.value = null
  sanitizedRemovals.value = []
  try {
    if (file.size > 10 * 1024 * 1024) {
      throw new Error('ファイルサイズが 10 MB を超えています')
    }

    const ext = file.name.split('.').pop()?.toLowerCase() ?? ''
    if (ext === 'svg') {
      const text = await file.text()
      const { sanitized, removed } = sanitizeSvg(text)
      if (!sanitized) throw new Error('SVG が解析できません: ' + removed.join(', '))
      sanitizedRemovals.value = removed
      const blob = new Blob([sanitized], { type: 'image/svg+xml' })
      importedPreviewUrl.value = URL.createObjectURL(blob)
      // SVG → 256px PNG にラスタライズして Rust 側ビルダー用に保持
      importedPngBytes.value = await rasterizeSvgToPng(sanitized, 256)
      importMessage.value = removed.length > 0
        ? `SVG を sanitize しました (除去: ${removed.length} 件)`
        : `SVG をインポートしました`
    } else if (ext === 'png') {
      // PNG は magic byte の弱検証のみ (89 50 4E 47)
      const fullBytes = new Uint8Array(await file.arrayBuffer())
      if (fullBytes.length < 8 || fullBytes[0] !== 0x89 || fullBytes[1] !== 0x50 || fullBytes[2] !== 0x4e || fullBytes[3] !== 0x47) {
        throw new Error('PNG ヘッダーが不正です (Magic Byte 不一致)')
      }
      importedPngBytes.value = fullBytes
      importedPreviewUrl.value = URL.createObjectURL(file)
      importMessage.value = 'PNG をインポートしました'
    } else {
      throw new Error(`未対応の拡張子: .${ext} (PNG / SVG のみ受付)`)
    }

    // 該当役割を partial → filled に
    filledRoles.add(activeRoleId.value)
    const map = filledSizesByRole.value[activeRoleId.value] ?? []
    if (!map.includes(activeSize.value)) {
      filledSizesByRole.value[activeRoleId.value] = [...map, activeSize.value]
    }

    // 役割マップに登録 (エクスポート時に使用)
    if (importedPngBytes.value) {
      assignedPng.value[activeRoleId.value] = importedPngBytes.value
      assignedHotspot.value[activeRoleId.value] = {
        x: hotspotX.value,
        y: hotspotY.value,
      }
    }
  } catch (err) {
    importMessage.value = `失敗: ${err instanceof Error ? err.message : String(err)}`
  } finally {
    importBusy.value = false
    if (fileInput.value) fileInput.value.value = ''
  }
}
</script>

<template>
  <div class="creator-host">
    <!-- ツールバー -->
    <div class="toolbar">
      <div class="bcrumb">
        <span class="crumb">Creator</span>
        <span class="sep">/</span>
        <span class="crumb active">
          Neon Glow
          <span class="draft-tag">v1.2.0 · draft</span>
        </span>
      </div>
      <div />
      <div class="tb-actions">
        <span v-if="hasKeystoreSigning" class="tag ok">
          <UiIcon name="Shield" :size="11" />SIGNED · ed25519
        </span>
        <span v-else class="tag" style="color: var(--rose); border-color: rgba(255,107,138,0.3);">
          <UiIcon name="Alert" :size="11" />未署名
        </span>
        <button
          class="btn ghost"
          :disabled="exportBusy || !arrowAssigned"
          title=".cursorpack を出力"
          @click="exportCursorpack({ sign: false })"
        >
          <span v-if="exportBusy" class="spinner" style="width: 13px; height: 13px" />
          <UiIcon v-else name="Export" :size="14" />
          {{ exportBusy ? '書き出し中...' : 'エクスポート' }}
        </button>
        <button
          v-if="hasKeystoreSigning"
          class="btn primary"
          :disabled="exportBusy || !arrowAssigned"
          @click="exportCursorpack({ sign: true })"
        >
          <UiIcon name="Shield" :size="14" />署名 & エクスポート
        </button>
        <button
          v-else
          class="btn primary"
          :disabled="buildBusy || !importedPngBytes"
          @click="buildAndSave"
        >
          <span v-if="buildBusy" class="spinner" style="width: 13px; height: 13px" />
          <UiIcon v-else name="Check" :size="14" />
          {{ buildBusy ? 'ビルド中...' : 'ビルド & 出力' }}
        </button>
      </div>
    </div>

    <!-- タブバー -->
    <div class="tabs">
      <button
        v-for="t in tabs"
        :key="t.id"
        :class="['tab', { active: activeTab === t.id }]"
        @click="activeTab = t.id"
      >
        {{ t.label }}
        <span v-if="t.count" class="num">{{ t.count }}</span>
      </button>
    </div>

    <!-- メタデータタブ -->
    <div v-if="activeTab === 'metadata'" class="metadata-pane">
      <div class="metadata-grid">
        <div class="prop-section">
          <div class="prop-head">テーマ情報</div>
          <div class="prop-body" style="padding: 4px 16px;">
            <SettingsRow label="名前 (日本語)" desc="必須。設定アプリやライブラリで表示されます">
              <input v-model="metaName" class="input" style="width: 280px" placeholder="Neon Glow" />
            </SettingsRow>
            <SettingsRow label="名前 (英語)" desc="任意。OS ロケールに応じて表示を切替">
              <input v-model="metaNameEn" class="input" style="width: 280px" placeholder="Neon Glow" />
            </SettingsRow>
            <SettingsRow label="作者" desc="GitHub username 推奨 (公式インデックス連携時に key_id 照合)">
              <input v-model="metaAuthor" class="input" style="width: 280px" placeholder="@username" />
            </SettingsRow>
            <SettingsRow label="バージョン" desc="SemVer (例: 1.2.0)">
              <input v-model="metaVersion" class="input mono" style="width: 140px" placeholder="1.0.0" />
            </SettingsRow>
            <SettingsRow label="OS 標準ポインター影を使用" desc="OFF にすると requires_os_shadow = false 相当 (画像に影が焼き込まれる場合)">
              <SettingsToggle v-model="shadowEnabled" />
            </SettingsRow>
          </div>
        </div>

        <div class="prop-section">
          <div class="prop-head">説明</div>
          <div class="prop-body" style="padding: 12px 16px;">
            <textarea
              v-model="metaDescription"
              class="input"
              rows="6"
              style="width: 100%; font-family: var(--font-body); resize: vertical;"
              placeholder="このテーマの特徴 / 推奨環境 / クレジット表記など"
            />
          </div>
        </div>

        <div class="prop-section">
          <div class="prop-head">エクスポート状況</div>
          <div class="prop-body" style="padding: 4px 16px;">
            <SettingsRow label="割り当て済みロール数">
              <span class="tag" :class="{ ok: arrowAssigned }">{{ assignedRoleCount }} / 17</span>
            </SettingsRow>
            <SettingsRow label="Arrow ロール (必須)">
              <span class="tag" :class="arrowAssigned ? 'ok' : ''">
                {{ arrowAssigned ? '割り当て済み' : '未割り当て' }}
              </span>
            </SettingsRow>
          </div>
        </div>
      </div>

      <Transition name="fade">
        <div v-if="exportMessage" class="import-banner" role="status">
          <UiIcon :name="exportMessage.startsWith('エクスポート失敗') ? 'Alert' : 'Check'" :size="13" />
          <span>{{ exportMessage }}</span>
          <button class="btn ghost" style="margin-left: auto; height: 24px" @click="exportMessage = null">
            <UiIcon name="X" :size="11" />
          </button>
        </div>
      </Transition>
    </div>

    <!-- 3 カラムグリッド (assign タブのみ) -->
    <div v-if="activeTab === 'assign'" class="creator-grid">
      <!-- 左: 役割リスト -->
      <div class="cpane left">
        <div class="pane-head">
          <h6>17 Roles</h6>
          <span class="tag">{{ filledCount }} / 17</span>
        </div>
        <div class="role-list">
          <RoleListItem
            v-for="(role, i) in CURSOR_ROLES"
            :key="role.id"
            :role="role"
            :index="i"
            :status="statusOf(role.id)"
            :active="activeRoleId === role.id"
            @select="selectRole"
          />
        </div>
      </div>

      <!-- 中央: エディタ -->
      <div class="editor">
        <div class="editor-head">
          <div>
            <h2>
              {{ activeRole.jp }}
              <span class="role-key">{{ activeRole.id }}</span>
            </h2>
            <div class="desc">
              <template v-if="isRequired(activeRole.id)">
                Windows のメイン操作カーソル。<b style="color: var(--accent)">必須</b>。解像度ごとに独立した画像を割り当て可能。
              </template>
              <template v-else>
                {{ activeRole.en }}。任意。未指定時は Windows 既定を継承します。
              </template>
            </div>
          </div>
          <div style="display: flex; gap: 6px">
            <button class="btn ghost" :disabled="importBusy" @click="pickImage">
              <span v-if="importBusy" class="spinner" style="width: 13px; height: 13px" />
              <UiIcon v-else name="Import" :size="13" />画像追加
            </button>
            <input
              ref="fileInput"
              type="file"
              accept=".png,.svg,image/png,image/svg+xml"
              hidden
              @change="onFileChange"
            >
          </div>
        </div>

        <!-- インポート結果メッセージ -->
        <Transition name="fade">
          <div v-if="importMessage" class="import-banner" role="status">
            <UiIcon :name="importMessage.startsWith('失敗') ? 'Alert' : 'Check'" :size="13" />
            <span>{{ importMessage }}</span>
            <button class="btn ghost" style="margin-left: auto; height: 24px" @click="importMessage = null">
              <UiIcon name="X" :size="11" />
            </button>
          </div>
        </Transition>

        <!-- ビルド結果メッセージ -->
        <Transition name="fade">
          <div v-if="buildMessage" class="import-banner" role="status">
            <UiIcon :name="buildMessage.startsWith('ビルド失敗') ? 'Alert' : 'Check'" :size="13" />
            <span>{{ buildMessage }}</span>
            <button class="btn ghost" style="margin-left: auto; height: 24px" @click="buildMessage = null">
              <UiIcon name="X" :size="11" />
            </button>
          </div>
        </Transition>

        <div class="canvas-area">
          <div class="canvas-stage">
            <!-- ビッグプレビュー -->
            <div class="bigpreview">
              <div class="crosshair-h" />
              <div class="crosshair-v" />
              <img
                v-if="importedPreviewUrl"
                :src="importedPreviewUrl"
                :alt="activeRole.jp"
                style="max-width: 90%; max-height: 90%; image-rendering: pixelated"
              >
              <CursorIcon
                v-else
                :role="activeRole.id"
                :size="90"
                style="color: var(--fg)"
              />
              <div class="hot" :style="{ left: '32%', top: '30%' }" />
              <div class="preview-meta tl">{{ activeSize }} × {{ activeSize }}</div>
              <div class="preview-meta tr">hotspot {{ hotspotX }},{{ hotspotY }}</div>
            </div>

            <!-- 6 サイズストリップ -->
            <SizeStrip
              :sizes="[...SIZES]"
              :filled-sizes="filledSizes"
              :active-size="activeSize"
              :role="activeRole.id"
              @select="selectSize"
            />

            <!-- リサンプル切替 -->
            <div class="resample-row">
              <span>RESAMPLE</span>
              <div class="btn-group">
                <button
                  v-for="mode in (['lanczos', 'nearest', 'auto'] as ResampleMode[])"
                  :key="mode"
                  :class="['btn', { active: resample === mode }]"
                  style="height: 26px; font-size: 11px"
                  @click="resample = mode"
                >
                  {{ mode === 'lanczos' ? 'Lanczos' : mode === 'nearest' ? 'Nearest' : 'Auto' }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 右: プロパティ -->
      <div class="cpane right">
        <!-- Hotspot -->
        <div class="prop-section">
          <div class="prop-head">
            Hotspot
            <span class="tag ok">px</span>
          </div>
          <div class="prop-body">
            <div class="prop-row">
              <label>X座標</label>
              <input v-model.number="hotspotX" type="number" class="input mono" min="0" />
            </div>
            <div class="prop-row">
              <label>Y座標</label>
              <input v-model.number="hotspotY" type="number" class="input mono" min="0" />
            </div>
            <div class="prop-row">
              <label>サイズ別</label>
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
          <div class="prop-head">アセット</div>
          <div class="prop-body">
            <div class="prop-row">
              <label>形式</label>
              <span class="tag">PNG · 24bit · α</span>
            </div>
            <div class="prop-row">
              <label>カラー</label>
              <div class="color-chips">
                <span class="cc" style="background: #7cf2d4" />
                <span class="cc" style="background: #0a0b0f" />
                <span class="cc" style="background: #ffffff" />
              </div>
            </div>
            <div class="prop-row">
              <label>影</label>
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
            Validation
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
            <div class="vrow"><span>resample-strategy</span><span class="dim">{{ resample }}3</span></div>
          </div>
        </div>
      </div>
    </div>

    <AppStatusbar
      :items="[
        { dot: true, text: '編集中: Neon Glow' },
        { text: `${filledCount}/17 役割 · ${sizesCovered}/6 解像度` },
        { text: '未保存の変更 3件' },
        { text: 'WebView2 132.0.2957' },
      ]"
    />
  </div>
</template>

<style scoped>
.creator-host {
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;
}

.draft-tag {
  color: var(--fg-mute);
  font-family: var(--font-mono);
  font-size: 10.5px;
  margin-left: 6px;
}

.pane-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.pane-head h6 {
  margin: 0;
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--fg-mute);
  font-weight: 500;
}

.role-key {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--fg-mute);
  font-weight: 400;
  margin-left: 8px;
}

.preview-meta {
  position: absolute;
  bottom: 10px;
  font-family: var(--font-mono);
  font-size: 10px;
}
.preview-meta.tl { left: 12px; color: var(--fg-mute); }
.preview-meta.tr { right: 12px; color: var(--accent); }

.resample-row {
  display: flex;
  gap: 8px;
  align-items: center;
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--fg-mute);
  letter-spacing: 0.04em;
}

.color-chips {
  display: flex;
  gap: 4px;
}
.cc {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: 1px solid var(--line);
}

.validation-body {
  gap: 8px;
  font-size: 11.5px;
  font-family: var(--font-mono);
  color: var(--fg-dim);
}
.vrow {
  display: flex;
  justify-content: space-between;
}
.vrow .ok { color: var(--accent); }
.vrow .warn { color: var(--amber); }
.vrow .dim { color: var(--fg-dim); }

.import-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  margin: 0 18px 8px;
  background: rgba(124, 242, 212, 0.06);
  border: 1px solid var(--accent-line);
  border-radius: 8px;
  font-size: 12px;
  color: var(--fg-dim);
}

.metadata-pane {
  flex: 1;
  overflow-y: auto;
  padding: 24px 28px 32px;
  background: radial-gradient(800px 600px at 50% 0%, rgba(124, 242, 212, 0.04), transparent 60%);
}
.metadata-grid {
  max-width: 760px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.creator-grid {
  flex: 1;
  min-height: 0;
}
</style>
