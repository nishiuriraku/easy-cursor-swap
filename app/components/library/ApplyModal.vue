<script setup lang="ts">
/**
 * テーマ適用確認モーダル。
 * design/screens.jsx の `CFApplyModal` を Vue 化したもの。
 *
 * - テーマメタ情報 (KV リスト) 表示
 * - 17 ロールのミニカーソル行
 * - カバレッジバーペア (overrides=mint / inherit=violet)
 * - フッターに署名情報 + キャンセル/適用ボタン
 *
 * 実際の IPC 呼び出しは親 (Library) で行う。当コンポーネントは emit のみ。
 */
import { computed, onMounted, ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import { useI18n } from '~/composables/useI18n'
import { invokeTauri } from '~/composables/useTauri'
import { useThemePreviews } from '~/composables/useThemePreviews'

const { t } = useI18n()

interface AccessibilityConflicts {
  mouse_sonar_enabled: boolean
  high_contrast_enabled: boolean
  cursor_base_size: number
  has_conflicts: boolean
}

const conflicts = ref<AccessibilityConflicts | null>(null)
const previewMap = ref<Record<string, string> | null>(null)
const { getMap } = useThemePreviews()

const conflictMessages = computed(() => {
  if (!conflicts.value || !conflicts.value.has_conflicts) return []
  const out: string[] = []
  if (conflicts.value.mouse_sonar_enabled) out.push(t('apply.conflictMouseSonar'))
  if (conflicts.value.high_contrast_enabled) out.push(t('apply.conflictHighContrast'))
  if (conflicts.value.cursor_base_size > 32) {
    out.push(t('apply.conflictCursorBaseSize', { size: conflicts.value.cursor_base_size }))
  }
  return out
})

onMounted(async () => {
  try {
    conflicts.value = await invokeTauri<AccessibilityConflicts>('get_accessibility_conflicts')
  } catch {
    // 取得失敗時はバナー非表示 (フェイルセーフ)
    conflicts.value = null
  }
  // テーマプレビューを取得 (失敗してもベクター描画にフォールバックする)
  if (props.theme.id) {
    previewMap.value = await getMap(props.theme.id)
  }
})

const props = defineProps<{
  theme: ThemeCardData
  /** 適用処理進行中フラグ (親が制御) */
  busy?: boolean
  /** Ed25519 署名済みなら key_id 短縮表示文字列 */
  signedKeyId?: string | null
}>()

const emit = defineEmits<{
  cancel: []
  confirm: [id: string]
}>()

const overridesCount = computed(() => props.theme.includedRoles.length)
const inheritCount = computed(() => 17 - overridesCount.value)
const overridesPct = computed(() => Math.round((overridesCount.value / 17) * 100))
const inheritPct = computed(() => 100 - overridesPct.value)

function onBackdropClick(e: MouseEvent) {
  // 内側のクリックは伝播してもキャンセルしない
  if (e.target === e.currentTarget && !props.busy) {
    emit('cancel')
  }
}
</script>

<template>
  <div class="modal-page" role="dialog" aria-modal="true" aria-labelledby="apply-modal-title" @click="onBackdropClick">
    <div class="modal" @click.stop>
      <!-- ヘッダー -->
      <div class="modal-head">
        <div class="modal-icon" aria-hidden="true"><UiIcon name="Pkg" :size="20" /></div>
        <div style="flex: 1; min-width: 0">
          <h2 id="apply-modal-title">{{ t('apply.title', { name: theme.name }) }}</h2>
          <p>{{ t('apply.description') }}</p>
        </div>
        <span v-if="signedKeyId" class="tag ok">
          <UiIcon name="Shield" :size="11" />{{ t('apply.signedTag') }}
        </span>
      </div>

      <!-- アクセシビリティ競合警告 -->
      <div v-if="conflictMessages.length > 0" class="a11y-banner" role="alert">
        <UiIcon name="AlertTriangle" :size="14" />
        <div>
          <strong>{{ t('apply.conflictTitle') }}</strong>
          <ul>
            <li v-for="(msg, i) in conflictMessages" :key="i">{{ msg }}</li>
          </ul>
        </div>
      </div>

      <!-- 本体 KV リスト -->
      <div class="modal-body">
        <div class="kvlist">
          <div class="kv">
            <label>{{ t('apply.themeLabel') }}</label>
            <div class="val">
              {{ theme.name }}
              <span class="sub">v{{ theme.version }} · @{{ theme.author ?? 'unknown' }}</span>
            </div>
          </div>

          <div class="kv">
            <label>{{ t('apply.coverage') }}</label>
            <div class="val" style="display: flex; align-items: center; gap: 10px">
              <div class="bar-pair" style="flex: 1; max-width: 180px">
                <i class="a" :style="{ width: overridesPct + '%' }" />
                <i class="b" :style="{ width: inheritPct + '%' }" />
              </div>
              <span style="font-family: var(--font-mono); font-size: 11px; color: var(--fg-dim)">
                <span style="color: var(--accent)">{{ overridesCount }}</span> {{ t('apply.overrides') }} ·
                <span style="color: var(--violet)">{{ inheritCount }}</span> {{ t('apply.inherit') }}
              </span>
            </div>
          </div>

          <div class="kv">
            <label>{{ t('apply.rolesLabel') }}</label>
            <div class="val">
              <div class="mini-row">
                <div
                  v-for="role in CURSOR_ROLES"
                  :key="role.id"
                  :class="['mini', { empty: !theme.includedRoles.includes(role.id) }]"
                  :title="role.jp"
                >
                  <template v-if="theme.includedRoles.includes(role.id)">
                    <img
                      v-if="previewMap && previewMap[role.id]"
                      :src="previewMap[role.id]"
                      :alt="role.jp"
                      class="mini-img"
                    >
                    <CursorIcon v-else :role="role.id" :size="14" />
                  </template>
                  <UiIcon v-else name="Plus" :size="10" />
                </div>
              </div>
            </div>
          </div>

          <div class="kv">
            <label>{{ t('apply.snapshot') }}</label>
            <div class="val" style="font-family: var(--font-mono); font-size: 12px; color: var(--fg-dim)">
              ~/.custom_cursors/_pending_apply.snapshot
            </div>
          </div>
        </div>
      </div>

      <!-- フッター -->
      <div class="modal-foot">
        <div class="left-note">
          <UiIcon name="Shield" :size="12" style="color: var(--accent)" />
          <span v-if="signedKeyId">{{ t('apply.signedNotice', { keyId: signedKeyId }) }}</span>
          <span v-else style="color: var(--rose)">{{ t('apply.unsignedNotice') }}</span>
        </div>
        <div class="actions">
          <button class="btn ghost" :disabled="busy" @click="emit('cancel')">
            {{ t('common.cancel') }}
          </button>
          <button class="btn primary" :disabled="busy" @click="emit('confirm', theme.id)">
            <span v-if="busy" class="spinner" style="width: 13px; height: 13px" />
            <UiIcon v-else name="Check" :size="13" />
            {{ busy ? t('apply.confirming') : t('apply.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mini-img {
  width: 16px;
  height: 16px;
  object-fit: contain;
  image-rendering: pixelated;
  image-rendering: crisp-edges;
}

.a11y-banner {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  margin: 0 20px 12px;
  padding: 10px 12px;
  border-radius: 6px;
  background: rgba(245, 158, 11, 0.1);
  border: 1px solid rgba(245, 158, 11, 0.35);
  color: var(--warning, #f59e0b);
  font-size: 12px;
  line-height: 1.5;
}

.a11y-banner strong {
  display: block;
  font-size: 12px;
  margin-bottom: 4px;
}

.a11y-banner ul {
  margin: 0;
  padding-left: 16px;
  color: var(--fg);
}
</style>
