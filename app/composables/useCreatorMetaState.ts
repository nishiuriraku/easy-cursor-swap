/**
 * Creator メタデータ入力欄の state を 1 箇所に集約する composable。
 *
 * 元は creator.vue 内で 5 個の ref + リセットロジック (resetCreator()) に分散していたが、
 * `<CreatorMetadataPane>` から v-model で受ける関係上 5 個全てが個別 ref として必要であり、
 * かつ resetCreator / dispatchBulkPaths / `?editPath` ハンドラから一括クリア・上書きが
 * 呼ばれるため、composable 化することで宣言とリセットを一行ずつにまとめられる。
 *
 * 注意: singleton ではなく per-call インスタンス。creator.vue で 1 回だけ呼ぶ前提。
 */
import { ref } from 'vue'
import { useI18n } from '~/composables/useI18n'

export function useCreatorMetaState() {
  const { t } = useI18n()

  const name = ref<string>(t('creator.untitledThemeName'))
  const nameEn = ref<string>('')
  const author = ref<string>('')
  const version = ref<string>('1.0.0')
  const description = ref<string>('')
  const shadowEnabled = ref<boolean>(false)

  /**
   * 全フィールドを「新規セッション開始時」の値に戻す。
   * resetCreator() からの呼び出しがメイン。`?editPath` 経由は別途
   * dispatchBulkPaths / parseCursorpack 側でメタを上書きする。
   */
  function reset() {
    name.value = t('creator.untitledThemeName')
    nameEn.value = ''
    author.value = ''
    version.value = '1.0.0'
    description.value = ''
    shadowEnabled.value = false
  }

  return { name, nameEn, author, version, description, shadowEnabled, reset }
}
