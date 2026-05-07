export default defineNuxtPlugin((nuxtApp) => {
  nuxtApp.vueApp.directive('click-outside', {
    mounted(el, binding) {
      // @ts-expect-error custom field on element
      el.__clickOutside__ = (e: Event) => {
        if (!el.contains(e.target as Node)) binding.value?.()
      }
      // @ts-expect-error custom field on element
      document.addEventListener('mousedown', el.__clickOutside__)
    },
    unmounted(el) {
      // @ts-expect-error custom field on element
      document.removeEventListener('mousedown', el.__clickOutside__)
    },
  })
})
