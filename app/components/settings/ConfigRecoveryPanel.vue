<script setup lang="ts">
/**
 * 設定復旧パネル (Phase 2-1 残: GUI 復旧フロー)
 *
 * config.corrupt.*.json が存在する場合のみ表示。
 * 選択したバックアップを config.json に上書きして設定を復旧する。
 */

const { t } = useI18n()

interface BackupInfo {
  file_name: string
  modified_utc: string
  size_bytes: number
  kind: 'corrupt'
}

const emit = defineEmits<{
  restored: []
}>()

const backups = ref<BackupInfo[]>([])
const busy = ref(false)
const message = ref<{ text: string; ok: boolean } | null>(null)

async function load() {
  try {
    backups.value = await invokeTauri<BackupInfo[]>('list_config_backups')
  } catch {
    backups.value = []
  }
}

async function restore(backup: BackupInfo) {
  const { ask } = await import('@tauri-apps/plugin-dialog')
  const ok = await ask(t('settings.recoveryAskMsg', { name: backup.file_name }), {
    title: t('settings.recoveryAskTitle'),
    kind: 'warning',
  })
  if (!ok) return

  busy.value = true
  message.value = null
  try {
    await invokeTauri<void>('restore_config_backup', { fileName: backup.file_name })
    message.value = { text: t('settings.recoverySuccess'), ok: true }
    emit('restored')
  } catch (err) {
    message.value = {
      text: t('settings.recoveryFail', {
        error: err instanceof Error ? err.message : String(err),
      }),
      ok: false,
    }
  } finally {
    busy.value = false
  }
}

function formatDate(iso: string) {
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

function kindLabel(_kind: string) {
  return t('settings.recoveryKindCorrupt')
}

onMounted(load)
</script>

<template>
  <div v-if="backups.length > 0" class="recovery-panel">
    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupRecovery') }}</div>
      <div class="prop-body">
        <p class="recovery-desc">{{ t('settings.recoveryDesc') }}</p>

        <div v-for="bk in backups" :key="bk.file_name" class="backup-row">
          <div class="backup-meta">
            <span class="backup-name">{{ bk.file_name }}</span>
            <span class="backup-kind" :class="bk.kind">{{ kindLabel(bk.kind) }}</span>
            <span class="backup-date">{{ formatDate(bk.modified_utc) }}</span>
            <span class="backup-size">{{
              t('settings.recoveryBytes', { size: bk.size_bytes.toLocaleString() })
            }}</span>
          </div>
          <button class="btn" :disabled="busy" @click="restore(bk)">
            <span v-if="busy" class="spinner" style="width: 12px; height: 12px" />
            {{ busy ? t('settings.recoveryRestoring') : t('settings.recoveryRestoreLabel') }}
          </button>
        </div>

        <div v-if="message" class="recovery-msg" :class="{ error: !message.ok }">
          {{ message.text }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.recovery-panel {
  @apply mt-1;
}

.prop-section {
  @apply mb-4 rounded-[8px] border border-line pb-1;
}

.prop-head {
  @apply border-b border-line px-4 pb-[9px] pt-2.5 text-[11px] font-semibold uppercase tracking-[0.06em] text-fg-mute;
}

.prop-body {
  @apply px-4 py-3;
}

.recovery-desc {
  @apply mb-3 mt-0 text-[13px] text-fg-dim;
}

.backup-row {
  @apply flex items-center gap-3 border-b border-line py-2.5;
}
.backup-row:last-of-type {
  @apply border-b-0;
}

.backup-meta {
  @apply flex min-w-0 flex-1 flex-col gap-[3px];
}

.backup-name {
  @apply overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[12px] text-fg;
}

.backup-kind {
  @apply w-fit rounded-[3px] px-1.5 py-px font-mono text-[11px];
}
.backup-kind.corrupt {
  @apply border;
  background: rgba(255, 107, 138, 0.1);
  color: #ffb8c5;
  border-color: rgba(255, 107, 138, 0.3);
}

.backup-date,
.backup-size {
  @apply text-[11px] text-fg-mute;
}

.recovery-msg {
  @apply mt-2.5 rounded-md border border-accent-line p-3 font-mono text-[11.5px] text-fg-dim;
  padding: 10px 12px;
  background: rgba(124, 242, 212, 0.06);
}
.recovery-msg.error {
  background: rgba(255, 107, 138, 0.06);
  border-color: rgba(255, 107, 138, 0.4);
  color: #ffb8c5;
}
</style>
