/**
 * v-click-outside ディレクティブ (Wave 2B / Task 5e)。
 *
 * 要素外側をクリック (mousedown) したとき binding の値 (コールバック) を呼ぶ。
 * listener ハンドルは `HTMLElement.__clickOutside__` に保持する
 * (型拡張は `app/types/dom.d.ts`)。
 *
 * `mounted` で `addEventListener`、`unmounted` で同じ参照を
 * `removeEventListener` するために要素ローカルに覚える必要がある。
 * `binding.value` は関数で Vue が差し替えることがあるため信頼できず、
 * closure 変数でもアンマウント時に参照が失われる。
 */
export default defineNuxtPlugin((nuxtApp) => {
  nuxtApp.vueApp.directive('click-outside', {
    mounted(el, binding) {
      el.__clickOutside__ = (e: Event) => {
        if (!el.contains(e.target as Node)) binding.value?.()
      }
      document.addEventListener('mousedown', el.__clickOutside__)
    },
    unmounted(el) {
      document.removeEventListener('mousedown', el.__clickOutside__)
    },
  })
})
