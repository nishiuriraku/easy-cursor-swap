<script setup lang="ts">
/**
 * OSS ライセンス一覧モーダル (設定 → About から開く)。
 *
 * 直接依存のみを表示する。推移的依存まで載せると 700+ になり実用的でないため、
 * 「アプリのトップレベル依存」「主要 Tauri プラグイン」「Rust の主要 crate」のみ。
 * すべて MIT / Apache-2.0 / BSD-3-Clause のいずれかなので個別ライセンス全文表示は省略し、
 * 各エントリの URL から確認できるようにする。
 *
 * 共通の `<UiModal>` shell を使用し、backdrop / Esc / focus trap / scroll lock は
 * shell 側に委譲する。
 */

const { t } = useI18n()

defineProps<{
  open: boolean
}>()
const emit = defineEmits<{
  close: []
}>()

interface OssEntry {
  name: string
  version?: string
  license: string
  url: string
}

const FRONTEND_DEPS: OssEntry[] = [
  { name: 'Vue.js', license: 'MIT', url: 'https://github.com/vuejs/core' },
  { name: 'Nuxt', license: 'MIT', url: 'https://github.com/nuxt/nuxt' },
  { name: 'Vue Router', license: 'MIT', url: 'https://github.com/vuejs/router' },
  { name: 'Tailwind CSS', license: 'MIT', url: 'https://github.com/tailwindlabs/tailwindcss' },
  {
    name: '@tauri-apps/api',
    license: 'MIT / Apache-2.0',
    url: 'https://github.com/tauri-apps/tauri',
  },
  {
    name: '@tauri-apps/plugin-dialog',
    license: 'MIT / Apache-2.0',
    url: 'https://github.com/tauri-apps/plugins-workspace',
  },
  {
    name: '@tauri-apps/plugin-notification',
    license: 'MIT / Apache-2.0',
    url: 'https://github.com/tauri-apps/plugins-workspace',
  },
  {
    name: '@tauri-apps/plugin-process',
    license: 'MIT / Apache-2.0',
    url: 'https://github.com/tauri-apps/plugins-workspace',
  },
  {
    name: '@tauri-apps/plugin-updater',
    license: 'MIT / Apache-2.0',
    url: 'https://github.com/tauri-apps/plugins-workspace',
  },
  { name: 'Vitest', license: 'MIT', url: 'https://github.com/vitest-dev/vitest' },
  { name: 'happy-dom', license: 'MIT', url: 'https://github.com/capricorn86/happy-dom' },
  { name: 'Prettier', license: 'MIT', url: 'https://github.com/prettier/prettier' },
  { name: '@resvg/resvg-js', license: 'MPL-2.0', url: 'https://github.com/yisibl/resvg-js' },
]

const BACKEND_DEPS: OssEntry[] = [
  { name: 'tauri', license: 'MIT / Apache-2.0', url: 'https://github.com/tauri-apps/tauri' },
  { name: 'serde', license: 'MIT / Apache-2.0', url: 'https://github.com/serde-rs/serde' },
  { name: 'tokio', license: 'MIT', url: 'https://github.com/tokio-rs/tokio' },
  { name: 'tracing', license: 'MIT', url: 'https://github.com/tokio-rs/tracing' },
  { name: 'image', license: 'MIT / Apache-2.0', url: 'https://github.com/image-rs/image' },
  { name: 'winreg', license: 'MIT', url: 'https://github.com/gentoo90/winreg-rs' },
  {
    name: 'windows-rs',
    license: 'MIT / Apache-2.0',
    url: 'https://github.com/microsoft/windows-rs',
  },
  {
    name: 'ed25519-dalek',
    license: 'BSD-3-Clause',
    url: 'https://github.com/dalek-cryptography/curve25519-dalek',
  },
  { name: 'sha2', license: 'MIT / Apache-2.0', url: 'https://github.com/RustCrypto/hashes' },
  {
    name: 'argon2',
    license: 'MIT / Apache-2.0',
    url: 'https://github.com/RustCrypto/password-hashes',
  },
  {
    name: 'chacha20poly1305',
    license: 'MIT / Apache-2.0',
    url: 'https://github.com/RustCrypto/AEADs',
  },
  { name: 'zip', license: 'MIT', url: 'https://github.com/zip-rs/zip2' },
  { name: 'reqwest', license: 'MIT / Apache-2.0', url: 'https://github.com/seanmonstar/reqwest' },
  { name: 'uuid', license: 'MIT / Apache-2.0', url: 'https://github.com/uuid-rs/uuid' },
  { name: 'chrono', license: 'MIT / Apache-2.0', url: 'https://github.com/chronotope/chrono' },
  { name: 'anyhow', license: 'MIT / Apache-2.0', url: 'https://github.com/dtolnay/anyhow' },
  { name: 'thiserror', license: 'MIT / Apache-2.0', url: 'https://github.com/dtolnay/thiserror' },
]

const openExternal = openExternalUrl

function close() {
  emit('close')
}
</script>

<template>
  <UiModal
    :open="open"
    :title="t('settings.ossModalTitle')"
    :description="t('settings.ossModalDesc')"
    icon="Pkg"
    size="md"
    @close="close"
  >
    <section>
      <h3 class="oss-section-title">{{ t('settings.ossSectionFrontend') }}</h3>
      <ul class="oss-list">
        <li v-for="dep in FRONTEND_DEPS" :key="dep.name" class="oss-row">
          <button class="oss-name" type="button" @click="openExternal(dep.url)">
            {{ dep.name }}
          </button>
          <span class="oss-license">{{ dep.license }}</span>
        </li>
      </ul>
    </section>

    <section>
      <h3 class="oss-section-title">{{ t('settings.ossSectionBackend') }}</h3>
      <ul class="oss-list">
        <li v-for="dep in BACKEND_DEPS" :key="dep.name" class="oss-row">
          <button class="oss-name" type="button" @click="openExternal(dep.url)">
            {{ dep.name }}
          </button>
          <span class="oss-license">{{ dep.license }}</span>
        </li>
      </ul>
    </section>

    <p class="oss-foot">
      {{ t('settings.ossModalFootnote') }}
    </p>

    <template #actions>
      <UiButton variant="ghost" @click="close">{{ t('common.close') }}</UiButton>
    </template>
  </UiModal>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.oss-section-title {
  @apply m-0 mb-2 text-[12px] font-semibold uppercase tracking-wider text-fg-mute;
}

.oss-list {
  @apply m-0 list-none p-0;
  display: grid;
  grid-template-columns: 1fr;
  gap: 4px;
}

.oss-row {
  @apply flex items-center justify-between gap-3 rounded-md border border-line px-3 py-2;
  background: rgba(255, 255, 255, 0.02);
}
:where(html.light) .oss-row {
  background: rgba(15, 20, 35, 0.02);
}

.oss-name {
  @apply truncate border-0 bg-transparent p-0 text-left font-medium text-fg underline decoration-dotted underline-offset-2;
  cursor: pointer;
}
.oss-name:hover {
  color: var(--accent);
}

.oss-license {
  @apply shrink-0 font-mono text-[11px] text-fg-mute;
}

.oss-foot {
  @apply m-0 text-[11.5px] text-fg-mute;
}

/* セクション間ギャップ (旧 .oss-body の gap:18px 相当) */
section + section {
  margin-top: 18px;
}
.oss-foot {
  margin-top: 18px;
}
</style>
