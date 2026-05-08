<script setup lang="ts">
/**
 * 「新規作成」を押したときに表示されるモーダル。
 *
 * 設計方針:
 *  - 1 枚のベース画像を選んで Arrow ロールに割り当て、その画像から 6 解像度を一括生成する
 *    (= 解像度ごとに別画像を選ぶワークフローはあくまで詳細設定オプション扱い)。
 *  - PNG / SVG / .cur / .ico の取り込みボタンは creator 本体と統合した共通ロジックを使う
 *    ため、このモーダルでは選択結果のバイト列とプレビュー URL だけを親に返す。
 *  - 「画像なしで開始」の逃げ道も用意して、空テンプレで編集を始めたいケースをカバー。
 */
import { ref, computed, onUnmounted } from 'vue'
import { sanitizeSvg } from '~/composables/sanitizeSvg'
import { invokeTauri } from '~/composables/useTauri'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

interface Props {
  open: boolean
}
defineProps<Props>()

const emit = defineEmits<{
  /** 画像が選択されて確定したとき。Arrow ロールに割り当てて編集画面へ遷移する想定。 */
  (e: 'confirm', payload: ConfirmPayload): void
  /** モーダルを閉じる (キャンセル / Esc / オーバーレイクリック)。 */
  (e: 'cancel'): void
  /** 画像なしで空のキャンバスから開始する。 */
  (e: 'start-empty'): void
}>()

export interface ConfirmPayload {
  png: Uint8Array
  /** Hotspot は後でエディタで微調整できるが、デフォルト 4,4 を渡しておく */
  hotspot: { x: number, y: number }
  /** UI プレビュー用の Object URL。creator 側で revoke する責務。 */
  previewUrl: string
  /** 「.cur/.ico 由来でホットスポット情報を含む」場合は true。 */
  fromCursorFile: boolean
  /** 元ファイルの primary サイズ (raster の場合)。SVG ラスタ時は 256。 */
  primarySize: number
}

const busy = ref(false)
const errorMsg = ref<string | null>(null)
const previewUrl = ref<string | null>(null)
const previewName = ref<string | null>(null)

/** モーダルを閉じるときに blob URL のリーク防止 */
function clearPreview() {
  if (previewUrl.value && previewUrl.value.startsWith('blob:')) {
    URL.revokeObjectURL(previewUrl.value)
  }
  previewUrl.value = null
  previewName.value = null
  errorMsg.value = null
}
onUnmounted(clearPreview)

const fileInput = ref<HTMLInputElement | null>(null)

/** PNG / SVG / .cur / .ico をまとめて選べる Tauri ダイアログを開く。 */
async function pickViaTauri() {
  busy.value = true
  errorMsg.value = null
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const picked = await open({
      multiple: false,
      filters: [{
        name: t('newTheme.fileFilterLabel'),
        extensions: ['png', 'svg', 'cur', 'ico'],
      }],
    })
    if (!picked || typeof picked !== 'string') return
    const ext = picked.split('.').pop()?.toLowerCase() ?? ''
    if (ext === 'cur' || ext === 'ico') {
      await loadCursorFile(picked)
    } else {
      await loadRasterOrSvgFromPath(picked, ext)
    }
  } catch (err) {
    errorMsg.value = err instanceof Error ? err.message : String(err)
  } finally {
    busy.value = false
  }
}

interface ImportCursorResult {
  isCur: boolean
  width: number
  height: number
  hotspotX: number
  hotspotY: number
  pngBytes: number[]
  availableSizes: number[]
}

async function loadCursorFile(path: string) {
  const result = await invokeTauri<ImportCursorResult>('import_cursor_file', { path })
  if (!result) throw new Error(t('newTheme.errorIpcEmpty'))
  const png = new Uint8Array(result.pngBytes)
  setPendingImage(png, path, result.width, true, { x: result.hotspotX, y: result.hotspotY })
}

async function loadRasterOrSvgFromPath(path: string, ext: string) {
  // Tauri 経由のファイル読み込み (Tauri の plugin-fs を経由するか、Rust 側 IPC を呼ぶ)。
  // 既存実装と揃えるため、ここでは `<input type="file">` のフォールバックも残す。
  // Tauri 開発時は plugin-fs 経由の方が確実なので、利用可能なら使う。
  try {
    const { readFile } = await import('@tauri-apps/plugin-fs')
    const data = await readFile(path)
    const bytes = data instanceof Uint8Array ? data : new Uint8Array(data)
    if (ext === 'svg') {
      const text = new TextDecoder().decode(bytes)
      const { sanitized } = sanitizeSvg(text)
      if (!sanitized) throw new Error(t('newTheme.errorSvgParse'))
      const png = await rasterizeSvgToPng(sanitized, 256)
      const url = URL.createObjectURL(new Blob([png.slice().buffer], { type: 'image/png' }))
      setPendingImageRaw(png, path.split(/[\\/]/).pop() ?? 'image.svg', 256, false, { x: 4, y: 4 }, url)
    } else {
      // PNG: magic-byte をざっと確認
      if (bytes.length < 8 || bytes[0] !== 0x89 || bytes[1] !== 0x50) {
        throw new Error(t('newTheme.errorPngMagic'))
      }
      setPendingImage(bytes, path, 256, false, { x: 4, y: 4 })
    }
  } catch (err) {
    // plugin-fs が無い環境 (純 Web プレビュー) → ファイル input にフォールバックさせる
    throw err
  }
}

function pickViaInput() {
  fileInput.value?.click()
}

async function onFileChange(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  busy.value = true
  errorMsg.value = null
  try {
    if (file.size > 10 * 1024 * 1024) throw new Error(t('newTheme.errorTooLarge'))
    const ext = file.name.split('.').pop()?.toLowerCase() ?? ''
    if (ext === 'svg') {
      const text = await file.text()
      const { sanitized } = sanitizeSvg(text)
      if (!sanitized) throw new Error(t('newTheme.errorSvgParse'))
      const png = await rasterizeSvgToPng(sanitized, 256)
      const url = URL.createObjectURL(new Blob([png.slice().buffer], { type: 'image/png' }))
      setPendingImageRaw(png, file.name, 256, false, { x: 4, y: 4 }, url)
    } else if (ext === 'png') {
      const bytes = new Uint8Array(await file.arrayBuffer())
      if (bytes.length < 8 || bytes[0] !== 0x89 || bytes[1] !== 0x50) {
        throw new Error(t('newTheme.errorPngMagic'))
      }
      const url = URL.createObjectURL(file)
      setPendingImageRaw(bytes, file.name, 256, false, { x: 4, y: 4 }, url)
    } else {
      throw new Error(t('newTheme.errorUnsupportedExt', { ext }))
    }
  } catch (err) {
    errorMsg.value = err instanceof Error ? err.message : String(err)
  } finally {
    busy.value = false
    if (fileInput.value) fileInput.value.value = ''
  }
}

function setPendingImage(
  png: Uint8Array,
  pathOrName: string,
  primarySize: number,
  fromCursorFile: boolean,
  hotspot: { x: number, y: number },
) {
  clearPreview()
  const url = URL.createObjectURL(new Blob([png.slice().buffer], { type: 'image/png' }))
  setPendingImageRaw(png, pathOrName.split(/[\\/]/).pop() ?? pathOrName, primarySize, fromCursorFile, hotspot, url)
}

function setPendingImageRaw(
  png: Uint8Array,
  filename: string,
  primarySize: number,
  fromCursorFile: boolean,
  hotspot: { x: number, y: number },
  url: string,
) {
  if (previewUrl.value && previewUrl.value.startsWith('blob:')) {
    URL.revokeObjectURL(previewUrl.value)
  }
  previewUrl.value = url
  previewName.value = filename
  pending.value = { png, hotspot, fromCursorFile, primarySize }
}

const pending = ref<{
  png: Uint8Array
  hotspot: { x: number, y: number }
  fromCursorFile: boolean
  primarySize: number
} | null>(null)

const canConfirm = computed(() => pending.value !== null && previewUrl.value !== null)

function confirm() {
  if (!pending.value || !previewUrl.value) return
  const url = previewUrl.value
  // url の所有権は親に渡す。clearPreview を呼ばずに参照だけ手放す。
  previewUrl.value = null
  emit('confirm', {
    png: pending.value.png,
    hotspot: pending.value.hotspot,
    previewUrl: url,
    fromCursorFile: pending.value.fromCursorFile,
    primarySize: pending.value.primarySize,
  })
  pending.value = null
  previewName.value = null
}

function cancel() {
  clearPreview()
  pending.value = null
  emit('cancel')
}

function startEmpty() {
  clearPreview()
  pending.value = null
  emit('start-empty')
}

/** sanitized SVG → 指定サイズの PNG (Canvas 経由) */
async function rasterizeSvgToPng(svgString: string, size: number): Promise<Uint8Array> {
  const blob = new Blob([svgString], { type: 'image/svg+xml' })
  const url = URL.createObjectURL(blob)
  try {
    const img = new Image()
    img.decoding = 'async'
    img.src = url
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve()
      img.onerror = () => reject(new Error(t('newTheme.errorSvgLoad')))
    })
    const canvas = document.createElement('canvas')
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext('2d')
    if (!ctx) throw new Error(t('newTheme.errorCanvas'))
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(img, 0, 0, size, size)
    const pngBlob: Blob = await new Promise((resolve, reject) => {
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error(t('newTheme.errorEncode')))), 'image/png')
    })
    return new Uint8Array(await pngBlob.arrayBuffer())
  } finally {
    URL.revokeObjectURL(url)
  }
}
</script>

<template>
  <div v-if="open" class="nt-overlay" @click.self="cancel" @keydown.esc="cancel">
    <div class="nt-modal" role="dialog" aria-modal="true" :aria-label="t('newTheme.title')">
      <header class="nt-head">
        <div>
          <div class="nt-eyebrow">CREATOR · NEW</div>
          <h3>{{ t('newTheme.title') }}</h3>
        </div>
        <button class="btn ghost" :aria-label="t('common.close')" @click="cancel">✕</button>
      </header>

      <div class="nt-body">
        <p class="nt-desc">{{ t('newTheme.description') }}</p>

        <div :class="['nt-drop', { ready: previewUrl, busy }]">
          <template v-if="!previewUrl">
            <div class="nt-drop-icon">
              <UiIcon name="Import" :size="28" />
            </div>
            <div class="nt-drop-title">{{ t('newTheme.dropTitle') }}</div>
            <div class="nt-drop-sub">{{ t('newTheme.dropSub') }}</div>
            <div class="nt-cta-row">
              <button class="btn primary" :disabled="busy" @click="pickViaTauri">
                <UiIcon name="Import" :size="13" />
                {{ t('newTheme.btnPick') }}
              </button>
              <button class="btn ghost" :disabled="busy" @click="pickViaInput">
                <UiIcon name="Brush" :size="13" />
                {{ t('newTheme.btnPickBrowser') }}
              </button>
              <input
                ref="fileInput"
                type="file"
                accept=".png,.svg,image/png,image/svg+xml"
                hidden
                @change="onFileChange"
              >
            </div>
          </template>
          <template v-else>
            <img :src="previewUrl" :alt="previewName ?? ''" class="nt-preview" />
            <div class="nt-preview-meta">
              <div class="nt-preview-name mono">{{ previewName }}</div>
              <button class="btn ghost" :disabled="busy" @click="clearPreview">
                <UiIcon name="X" :size="11" />{{ t('newTheme.changeImage') }}
              </button>
            </div>
          </template>
        </div>

        <div v-if="errorMsg" class="nt-error" role="alert">
          <UiIcon name="Alert" :size="13" />
          <span>{{ errorMsg }}</span>
        </div>

        <p class="nt-tip">
          <UiIcon name="Shield" :size="11" />
          {{ t('newTheme.tip') }}
        </p>
      </div>

      <footer class="nt-foot">
        <button class="btn ghost" @click="startEmpty">
          {{ t('newTheme.startEmpty') }}
        </button>
        <div class="nt-foot-r">
          <button class="btn ghost" @click="cancel">{{ t('common.cancel') }}</button>
          <button class="btn primary" :disabled="!canConfirm" @click="confirm">
            <UiIcon name="Check" :size="13" />
            {{ t('newTheme.confirm') }}
          </button>
        </div>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.nt-overlay {
  position: fixed; inset: 0;
  background: rgba(10, 11, 15, 0.7);
  backdrop-filter: blur(2px);
  display: flex; align-items: center; justify-content: center;
  z-index: 100;
}
.nt-modal {
  background: var(--bg-1, #14161c);
  border: 1px solid var(--line);
  border-radius: 12px;
  width: min(560px, 96vw);
  max-height: 90vh;
  display: flex; flex-direction: column;
  box-shadow: 0 30px 60px rgba(0, 0, 0, 0.45);
}
.nt-head, .nt-foot {
  padding: 14px 20px;
  display: flex; align-items: flex-start; justify-content: space-between;
}
.nt-head { border-bottom: 1px solid var(--line); }
.nt-foot { border-top: 1px solid var(--line); align-items: center; }
.nt-foot-r { display: flex; gap: 8px; }
.nt-head h3 { margin: 4px 0 0; font-size: 16px; font-weight: 600; }
.nt-eyebrow {
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.16em;
  color: var(--accent);
}
.nt-body { padding: 14px 20px 18px; overflow-y: auto; }
.nt-desc { font-size: 12.5px; color: var(--fg-dim); margin: 0 0 14px; line-height: 1.55; }

.nt-drop {
  border: 1.5px dashed var(--line);
  border-radius: 12px;
  padding: 24px;
  display: flex; flex-direction: column; align-items: center; gap: 8px;
  background: rgba(124, 242, 212, 0.02);
  transition: border-color 160ms ease, background 160ms ease;
}
.nt-drop.ready {
  border-style: solid;
  border-color: var(--accent-line);
  background: rgba(124, 242, 212, 0.05);
}
.nt-drop.busy { opacity: 0.6; pointer-events: none; }
.nt-drop-icon { color: var(--accent); }
.nt-drop-title { font-size: 13px; font-weight: 600; }
.nt-drop-sub { font-size: 11.5px; color: var(--fg-mute); }
.nt-cta-row { display: flex; gap: 8px; margin-top: 10px; flex-wrap: wrap; justify-content: center; }
.nt-preview {
  max-width: 128px; max-height: 128px;
  image-rendering: pixelated;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.03);
  padding: 8px;
}
.nt-preview-meta {
  display: flex; align-items: center; gap: 12px; margin-top: 8px;
  font-size: 11.5px; color: var(--fg-dim);
}
.nt-preview-name { max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.nt-error {
  display: flex; align-items: center; gap: 8px;
  margin-top: 12px; padding: 8px 12px;
  border: 1px solid rgba(255, 107, 138, 0.35);
  border-radius: 8px;
  font-size: 12px;
  color: var(--rose, #ff6b8a);
  background: rgba(255, 107, 138, 0.06);
}
.nt-tip {
  display: flex; align-items: center; gap: 6px;
  margin: 14px 0 0; font-size: 11px; color: var(--fg-mute);
}
</style>
