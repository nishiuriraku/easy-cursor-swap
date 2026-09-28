/**
 * Creator の破棄ガード (P08a Step 2 / C1)。
 *
 * `creator.vue` から「未保存編集の判定 + 破棄ダイアログ + route guard +
 * 保存後遷移スケジュール」を移動したもの。
 *
 * `onBeforeRouteLeave` は `vue-router` から明示 import する (auto-import でも
 * 動くが、vitest で `vi.mock('vue-router')` しやすくするため)。
 */
import type { ComputedRef, Ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'

export interface CreatorDiscardGuardDeps {
  stage: Ref<'start' | 'editing'>
  assignedRoleCount: ComputedRef<number>
  isMetaDirty: ComputedRef<boolean>
  onReset: () => void
  /** 画面遷移関数。省略時は Nuxt の `navigateTo` (vitest では global に無いためテスト時は必ず渡す)。 */
  navigate?: (path: string) => unknown
}

export function useCreatorDiscardGuard(deps: CreatorDiscardGuardDeps) {
  const { stage, assignedRoleCount, isMetaDirty, onReset } = deps
  const navigate = deps.navigate ?? ((path: string) => navigateTo(path))

  /**
   * 編集破棄ダイアログのガード判定。
   * 編集ステージにいて、アセット割り当て または メタ入力 のどちらかがあれば true。
   * Clear ボタン / 画面遷移どちらの経路でもこの判定でダイアログ表示を分岐する。
   */
  const hasUnsavedEdits = computed(() => {
    if (stage.value !== 'editing') return false
    if (assignedRoleCount.value > 0) return true
    return isMetaDirty.value
  })

  /**
   * 編集破棄ダイアログの開閉と「破棄後に何をするか」を保持する。
   * - mode='clear':   confirm 後に resetCreator() を実行
   * - mode='navigate': confirm 後に Vue Router の next() を実行 (cancel 時は next(false))
   *
   * `pendingNavigation` は onBeforeRouteLeave から渡された next() のサンクで、
   * confirm / cancel 経路の両方で必ず呼び切る (放置すると router がフリーズする)。
   */
  const discardDialogOpen = ref(false)
  const discardDialogMode = ref<'clear' | 'navigate'>('clear')
  let pendingNavigation: ((proceed: boolean) => void) | null = null

  /**
   * 保存成功直後にライブラリへ自動遷移する際、破棄ダイアログをスキップするためのフラグ。
   *  - true の間: `onBeforeRouteLeave` は `hasUnsavedEdits` を見ずに即 next() する。
   *  - 保存直後にだけ立て、遷移完了後に false に戻す (unmount でリセットされるので明示的な
   *    後始末は不要だが、保存後に同一ページ内へ遷移しない再エントリも考慮して reset する)。
   */
  const bypassUnsavedGuard = ref(false)

  /** 保存後遷移のためにスケジュール済みの setTimeout ハンドル。unmount で確実にクリアする。 */
  let postSaveNavTimer: ReturnType<typeof setTimeout> | null = null

  function requestReset() {
    if (!hasUnsavedEdits.value) {
      onReset()
      return
    }
    discardDialogMode.value = 'clear'
    pendingNavigation = null
    discardDialogOpen.value = true
  }

  function onDiscardConfirm() {
    discardDialogOpen.value = false
    const navigation = pendingNavigation
    pendingNavigation = null
    if (navigation) {
      navigation(true)
    } else {
      onReset()
    }
  }

  function onDiscardCancel() {
    discardDialogOpen.value = false
    const navigation = pendingNavigation
    pendingNavigation = null
    if (navigation) navigation(false)
  }

  // サイドバー / ブラウザバック相当の遷移をガードする。
  // Vue Router の onBeforeRouteLeave は next(false) で離脱をキャンセルできる。
  onBeforeRouteLeave((_to, _from, next) => {
    // 保存直後の自動遷移は破棄ダイアログをスキップする (assigned/メタが残っていても
    // 既にバックエンドへ保存済みなので破棄リスクは無い)。
    if (bypassUnsavedGuard.value) {
      next()
      return
    }
    if (!hasUnsavedEdits.value) {
      next()
      return
    }
    discardDialogMode.value = 'navigate'
    pendingNavigation = (proceed) => next(proceed)
    discardDialogOpen.value = true
  })

  /**
   * 保存成功後のライブラリ自動遷移をスケジュールする。
   * トースト視認のため ~1 秒遅延し、遷移時は破棄確認をスキップする。
   */
  function scheduleNavigateAfterSave(path: string, delayMs: number) {
    if (postSaveNavTimer) clearTimeout(postSaveNavTimer)
    postSaveNavTimer = setTimeout(() => {
      postSaveNavTimer = null
      bypassUnsavedGuard.value = true
      void navigate(path)
    }, delayMs)
  }

  onUnmounted(() => {
    if (postSaveNavTimer) {
      clearTimeout(postSaveNavTimer)
      postSaveNavTimer = null
    }
  })

  return {
    hasUnsavedEdits,
    discardDialogOpen,
    discardDialogMode,
    bypassUnsavedGuard,
    requestReset,
    onDiscardConfirm,
    onDiscardCancel,
    scheduleNavigateAfterSave,
  }
}
