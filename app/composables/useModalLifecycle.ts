/**
 * Modal SFC で重複していた Teleport 配下の lifecycle 管理を共通化する composable。
 *
 * - `<Teleport to="body">` 配下の Vue lifecycle と DOM 側の状態 (body.overflow,
 *   keydown listener) を確実に同期させる
 * - 複数 modal がスタックしている状態でも `body.overflow` の復元順序を壊さない
 *   よう、モジュールスコープの counter で multi-modal locking を扱う
 *
 * 経緯: ThemeDetailModal / MarketplaceDetailModal / OssLicenseModal で同型の
 * `watch(isOpen) → body.overflow lock + keydown Escape` ブロックが 3 重複しており、
 * かつ各実装が独立して `prevOverflow` を握っていたため、A→B→A close 時に lock
 * が永続化する潜在バグがあった (audit D28-DUP-001)。
 */
import { onBeforeUnmount, watch, type Ref } from 'vue'

/**
 * モジュール singleton: 複数の modal が同時に開いた時の scroll lock 重ね合わせを管理する。
 * `lockBodyScroll()` を呼ぶたびに counter を進め、最後の `unlockBodyScroll()` で
 * 元の `body.overflow` を復元する (locker の入れ子を安全にする)。
 */
let lockCounter = 0
let savedOverflow: string | null = null

function lockBodyScroll(): void {
  if (typeof document === 'undefined') return
  if (lockCounter === 0) {
    savedOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
  }
  lockCounter += 1
}

function unlockBodyScroll(): void {
  if (typeof document === 'undefined') return
  lockCounter = Math.max(0, lockCounter - 1)
  if (lockCounter === 0 && savedOverflow !== null) {
    document.body.style.overflow = savedOverflow
    savedOverflow = null
  }
}

export interface ModalLifecycleOptions {
  /** モーダルの開閉状態 (reactive)。true で lifecycle を有効化、false で解除。 */
  open: Ref<boolean>
  /** Escape キー押下時に呼ばれるハンドラ。通常 `emit('close')` を呼ぶ。 */
  onClose: () => void
  /** body スクロールをロックするか (default: true)。 */
  lockScroll?: boolean
  /** Escape キー購読の有効化 (default: true)。 */
  closeOnEscape?: boolean
}

/**
 * 利用例:
 * ```ts
 * const isOpen = computed(() => props.theme !== null)
 * useModalLifecycle({ open: isOpen, onClose: () => emit('close') })
 * ```
 *
 * 呼出側は引き続き `<Teleport to="body">` でレンダリング階層を制御する。
 * 本 composable は scroll lock / Esc 購読 / cleanup のみを引き受ける。
 */
export function useModalLifecycle(opts: ModalLifecycleOptions): void {
  const lockScroll = opts.lockScroll ?? true
  const closeOnEscape = opts.closeOnEscape ?? true

  // この composable インスタンスが現在「アクティブ (lifecycle ON)」か。
  // watch のフリップで複数回 activate/deactivate されても安全に多重 lock しない
  // ためのガード。
  let active = false

  function onKeydown(e: KeyboardEvent) {
    if (closeOnEscape && e.key === 'Escape') {
      e.preventDefault()
      opts.onClose()
    }
  }

  function activate(): void {
    if (typeof document === 'undefined' || active) return
    active = true
    if (lockScroll) lockBodyScroll()
    if (closeOnEscape) document.addEventListener('keydown', onKeydown)
  }

  function deactivate(): void {
    if (typeof document === 'undefined' || !active) return
    active = false
    if (lockScroll) unlockBodyScroll()
    if (closeOnEscape) document.removeEventListener('keydown', onKeydown)
  }

  watch(
    opts.open,
    (open) => {
      if (open) activate()
      else deactivate()
    },
    { immediate: true },
  )

  // モーダルが開いたままアンマウントされた場合 (タブ切替などで親が消えるケース) に
  // body スクロールがロックされたまま残らないよう、必ず復元する。
  onBeforeUnmount(deactivate)
}
