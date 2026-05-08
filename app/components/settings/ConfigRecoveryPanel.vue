<script setup lang="ts">
/**
 * 設定復旧パネル (Phase 2-1 残: GUI 復旧フロー)
 *
 * config.bak.v*.json / config.corrupt.*.json が存在する場合のみ表示。
 * 選択したバックアップを config.json に上書きして設定を復旧する。
 */
import { onMounted, ref } from 'vue'
import { invokeTauri } from '~/composables/useTauri'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

interface BackupInfo {
  file_name: string
  modified_utc: string
  size_bytes: number
  kind: 'versioned' | 'corrupt'
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

function kindLabel(kind: string) {
  return kind === 'versioned'
    ? t('settings.recoveryKindVersioned')
    : t('settings.recoveryKindCorrupt')
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
.recovery-panel {
  margin-top: 4px;
}

.prop-section {
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 0 0 4px;
  margin-bottom: 16px;
}

.prop-head {
  padding: 10px 16px 9px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--fg-mute);
  border-bottom: 1px solid var(--line);
}

.prop-body {
  padding: 12px 16px;
}

.recovery-desc {
  margin: 0 0 12px;
  font-size: 13px;
  color: var(--fg-dim);
}

.backup-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--line);
}
.backup-row:last-of-type {
  border-bottom: none;
}

.backup-meta {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.backup-name {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.backup-kind {
  font-size: 11px;
  font-family: var(--font-mono);
  padding: 1px 6px;
  border-radius: 3px;
  width: fit-content;
}
.backup-kind.versioned {
  background: rgba(124, 242, 212, 0.12);
  color: var(--accent);
  border: 1px solid var(--accent-line);
}
.backup-kind.corrupt {
  background: rgba(255, 107, 138, 0.1);
  color: #ffb8c5;
  border: 1px solid rgba(255, 107, 138, 0.3);
}

.backup-date,
.backup-size {
  font-size: 11px;
  color: var(--fg-mute);
}

.recovery-msg {
  margin-top: 10px;
  padding: 10px 12px;
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--fg-dim);
  background: rgba(124, 242, 212, 0.06);
  border: 1px solid var(--accent-line);
  border-radius: 6px;
}
.recovery-msg.error {
  background: rgba(255, 107, 138, 0.06);
  border-color: rgba(255, 107, 138, 0.4);
  color: #ffb8c5;
}
</style>
