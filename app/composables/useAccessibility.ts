/**
 * Windows アクセシビリティ競合・カーソルサイズ状態 (`get_accessibility_conflicts` IPC) の
 * 薄いラッパ。ApplyModal / settings.vue が直接 invoke していたのを集約する。
 * 型は `src-tauri/src/accessibility.rs::AccessibilityConflicts` を mirror する。
 */
export interface AccessibilityConflicts {
  mouse_sonar_enabled: boolean
  high_contrast_enabled: boolean
  cursor_base_size: number
  cursor_size_slider: number
  cursor_type: number
  has_conflicts: boolean
}

export function useAccessibility() {
  /** 現在のアクセシビリティ競合・カーソルサイズ状態を取得する。失敗時は null。 */
  function getAccessibilityConflicts(): Promise<AccessibilityConflicts | null> {
    return invokeTauri<AccessibilityConflicts>('get_accessibility_conflicts')
  }
  return { getAccessibilityConflicts }
}
