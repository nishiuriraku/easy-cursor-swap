<script setup lang="ts">
/**
 * テーマ詳細ドロワー (インライン展開)
 *
 * design/library-detail.jsx の `ThemeDetailDrawer` を Vue 化したもの。
 * ThemeCard 内のシェブロンを押すと、カードが 2 列分の幅にスパンして
 * このドロワーが展開する設計 (CSS: `.card.td-open`)。
 *
 * 5 ペイン構成:
 *   1. 説明 + チェンジログ (左)
 *   2. 17 ロールカバレッジ + ライブプレビュー (右)
 *   3. パッケージ / 署名 / 使用統計 / ペアリング (帯)
 *   4. アクション群 (フッター)
 *
 * 説明文・チェンジログ・ペアリング等のデータはまだ Rust 側に IPC 経路が
 * ないので、利用可能な ThemeCardData の値からフォールバック表示する。
 * (将来的に `get_theme_detail` IPC を追加する想定)
 *
 * Windows システムスキーム (`kind: 'system'`) は署名・統計・編集系操作を
 * 全て隠す。
 */
import { computed, ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
  /** 役割名 → PNG Object URL のマップ。null のときは UiIcon のフォールバックを表示。 */
  previewMap: Record<string, string> | null
}>()

const emit = defineEmits<{
  apply: [id: string]
  close: []
  edit: [id: string]
  duplicate: [id: string]
  exportPack: [id: string]
  delete: [id: string]
  openSource: [id: string]
}>()

const isSystem = computed(() => props.theme.kind === 'system')

// 17 ロールのうちアクティブにプレビュー中のロール
const activeRole = ref<string>(props.theme.includedRoles[0] ?? 'Arrow')

const includedSet = computed(() => new Set(props.theme.includedRoles))
const coverage = computed(() => props.theme.includedRoles.length)

const activeRoleDef = computed(
  () => CURSOR_ROLES.find((r) => r.id === activeRole.value) ?? CURSOR_ROLES[0]!,
)
const activeIncluded = computed(() => includedSet.value.has(activeRole.value))
const activePreviewUrl = computed(() => props.previewMap?.[activeRole.value] ?? null)

function selectRole(id: string) {
  activeRole.value = id
}

const description = computed(() => {
  if (isSystem.value) {
    return 'Windows のマウスのプロパティに保存された配色スキームです。EasyCursorSwap では適用のみ可能で、編集・エクスポート・署名検証は行いません。'
  }
  return `カバレッジ ${coverage.value}/17 役割。詳細な説明は将来のリリースで .cursorpack のメタデータから読み込む予定です。`
})

const tags = computed<string[]>(() => {
  const out: string[] = []
  if (isSystem.value) out.push('system')
  else out.push('cursor')
  if (coverage.value === 17) out.push('complete')
  if (props.theme.applyCount > 0) out.push('used')
  return out
})
</script>

<template>
  <div class="td-drawer">
    <!-- 上段: 説明 / チェンジログ | 17 ロールカバレッジ -->
    <div class="td-grid">
      <section class="td-pane">
        <header class="td-pane-h">
          <span class="td-pane-k">DESCRIPTION</span>
        </header>
        <p class="td-desc">{{ description }}</p>

        <div class="td-tags">
          <span v-for="tag in tags" :key="tag" class="td-tag">{{ tag }}</span>
          <span v-if="!isSystem" class="td-tag td-tag-on">
            <UiIcon name="Shield" :size="10" />signed
          </span>
        </div>

        <header v-if="!isSystem" class="td-pane-h" style="margin-top: 18px">
          <span class="td-pane-k">VERSION</span>
        </header>
        <ul v-if="!isSystem" class="td-changelog">
          <li class="current">
            <span class="td-cv">v{{ theme.version }}</span>
            <span class="td-cd">{{ theme.date.slice(0, 10) }}</span>
            <span class="td-cm">{{ t('themePicker.latestVersionNote') }}</span>
          </li>
        </ul>
      </section>

      <section class="td-pane">
        <header class="td-pane-h">
          <span class="td-pane-k">ROLE COVERAGE</span>
          <span class="td-pane-r">
            <span style="color: var(--accent)">{{ coverage }}</span>
            <span style="color: var(--fg-faint)">/</span>
            <span>17</span>
          </span>
        </header>

        <div class="td-cov">
          <div class="td-rolegrid">
            <button
              v-for="role in CURSOR_ROLES"
              :key="role.id"
              :class="[
                'td-rolebtn',
                {
                  empty: !includedSet.has(role.id),
                  active: activeRole === role.id,
                },
              ]"
              :title="role.jp"
              @click="selectRole(role.id)"
            >
              <CursorIcon
                v-if="includedSet.has(role.id)"
                :role="role.id"
                :size="14"
              />
              <span v-else class="td-rb-x">×</span>
            </button>
          </div>

          <div class="td-rolepreview">
            <div class="td-rp-stage">
              <template v-if="activeIncluded">
                <img
                  v-if="activePreviewUrl"
                  :src="activePreviewUrl"
                  :alt="activeRoleDef.jp"
                  draggable="false"
                  style="width: 64px; height: 64px; image-rendering: pixelated"
                />
                <CursorIcon
                  v-else
                  :role="activeRoleDef.id"
                  :size="64"
                  style="color: var(--fg)"
                />
                <span class="td-rp-hot" />
              </template>
              <div v-else class="td-rp-missing">
                <UiIcon name="Alert" :size="20" />
                <span>{{ t('themePicker.roleMissing') }}</span>
              </div>
            </div>
            <div class="td-rp-meta">
              <div class="td-rp-name">{{ activeRoleDef.jp }}</div>
              <div class="td-rp-key"><code>{{ activeRoleDef.id }}</code></div>
            </div>
          </div>
        </div>
      </section>
    </div>

    <!-- 中段: パッケージ情報帯 (system では一部のみ) -->
    <div class="td-strip">
      <div class="td-cell">
        <div class="td-cell-k">PACKAGE</div>
        <div class="td-cell-v mono">
          {{ isSystem ? '— (system scheme)' : `${theme.id.slice(0, 8)}.cursorpack` }}
        </div>
        <div class="td-cell-sub">
          <span>{{ coverage }} roles</span>
          <span class="td-dot">·</span>
          <span>{{ isSystem ? 'system' : 'schema v3.2' }}</span>
        </div>
      </div>

      <div class="td-cell">
        <div class="td-cell-k">
          SIGNATURE
          <span v-if="isSystem" class="td-pill">N/A</span>
          <span v-else class="td-pill ok">
            <UiIcon name="Shield" :size="9" />Ed25519
          </span>
        </div>
        <div class="td-cell-v mono trunc">
          {{ isSystem ? 'Windows OS スキーム' : 'key_id 検証中…' }}
        </div>
        <div class="td-cell-sub mono">
          {{ isSystem ? 'EasyCursorSwap の署名対象外' : '適用時にハッシュ照合' }}
        </div>
      </div>

      <div class="td-cell">
        <div class="td-cell-k">USAGE</div>
        <div class="td-cell-v">
          <span :style="{ color: theme.applyCount > 0 ? 'var(--fg)' : 'var(--fg-mute)' }">
            {{ theme.applyCount }}
          </span>
          <span style="color: var(--fg-dim); font-size: 12px; font-weight: 400; margin-left: 4px">
            回適用
          </span>
        </div>
        <div class="td-cell-sub">
          <span>{{ theme.isActive ? '現在適用中' : '未適用' }}</span>
        </div>
      </div>

      <div class="td-cell">
        <div class="td-cell-k">SOURCE</div>
        <div class="td-cell-v">
          <UiIcon
            :name="isSystem ? 'Globe' : 'Pkg'"
            :size="11"
            :style="`color: var(${isSystem ? '--violet' : '--accent'}); margin-right: 6px`"
          />
          {{ isSystem ? 'HKCU\\Cursors\\Schemes' : `@${theme.author ?? 'unknown'}` }}
        </div>
        <div class="td-cell-sub">
          <span>{{ isSystem ? 'OS レジストリ' : `v${theme.version}` }}</span>
        </div>
      </div>
    </div>

    <!-- 下段: アクションレール -->
    <footer class="td-foot">
      <div class="td-foot-l">
        <template v-if="!isSystem">
          <button
            class="td-act"
            :aria-label="`${theme.name} を Creator で編集`"
            @click="emit('edit', theme.id)"
          >
            <UiIcon name="Brush" :size="13" />Creator で編集
          </button>
          <button
            class="td-act"
            :aria-label="`${theme.name} をエクスポート`"
            @click="emit('exportPack', theme.id)"
          >
            <UiIcon name="Export" :size="13" />エクスポート
          </button>
          <button
            class="td-act"
            :aria-label="`${theme.name} を複製`"
            @click="emit('duplicate', theme.id)"
          >
            <UiIcon name="Plus" :size="13" />複製
          </button>
          <button
            class="td-act danger"
            :aria-label="`${theme.name} を削除`"
            @click="emit('delete', theme.id)"
          >
            削除
          </button>
        </template>
        <template v-else>
          <!--
            システムスキームは編集・複製・削除はできないが、`.cursorpack`
            として書き出して別環境へ持ち運ぶことはできる。Rust 側の
            `export_windows_scheme_as_cursorpack` が `%SystemRoot%\cursors\*`
            を読み取って zip 化するため、ローカルディレクトリは不要。
          -->
          <button
            class="td-act"
            :aria-label="`${theme.name} を .cursorpack としてエクスポート`"
            @click="emit('exportPack', theme.id)"
          >
            <UiIcon name="Export" :size="13" />.cursorpack に書き出し
          </button>
          <span class="td-source mono">
            <UiIcon name="Globe" :size="11" />システムスキームは編集・複製不可
          </span>
        </template>
      </div>
      <div class="td-foot-r">
        <button
          v-if="theme.isActive"
          class="btn"
          disabled
          style="opacity: 0.6; cursor: default; height: 32px"
        >
          <UiIcon name="Check" :size="13" />適用中
        </button>
        <button
          v-else
          class="btn primary"
          style="height: 32px"
          @click="emit('apply', theme.id)"
        >
          テーマを適用
        </button>
      </div>
    </footer>
  </div>
</template>
