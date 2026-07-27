/**
 * DOM の正準拡張宣言 (Wave 2B / Task 5e)。
 *
 * Vue のカスタムディレクティブが要素に直接フィールドを割り当てる際、
 * `HTMLElement` の組み込み型に存在しないプロパティのため従来は
 * `@ts-expect-error custom field on element` を付けて凌いでいた。
 *
 * グローバル `declare global { interface HTMLElement { ... } }` で
 * 拡張することで、`@ts-expect-error` を全廃し、`vue-tsc` を clean な
 * 状態に保ちつつ、ディレクティブ実装側は普通に代入 / 参照できる。
 *
 * Nuxt 4 は app 配下の `.d.ts` を `.nuxt/tsconfig.app.json` の `include`
 * glob (`../app` 以下すべて) 経由で拾うため、追加設定なしで全 SFC / composable に
 * この拡張が反映される。
 * (注: glob 表記そのままだとブロックコメントが途中終端するため文章で記載)
 */
export {}

declare global {
  interface HTMLElement {
    /**
     * `v-click-outside` ディレクティブがバインド時に設定する listener ハンドル。
     * unmounted 時に同じ参照を使って `removeEventListener` するため、
     * 要素ローカルに保持する必要がある (クロージャ変数では参照できない)。
     */
    __clickOutside__?: (event: Event) => void
  }
}
