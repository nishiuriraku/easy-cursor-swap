/**
 * @vitest-environment happy-dom
 *
 * BulkImportProgressOverlay (LD5) — resolve/parse 中のオーバーレイ。
 * progress 払い出しから StageStepper / Progress / message / cancel への配線を検証する。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import BulkImportProgressOverlay from '../BulkImportProgressOverlay.vue'

const stubs = {
  UiSpinner: { props: ['size', 'label'], template: '<span class="spinner-stub" />' },
  UiIcon: { props: ['name', 'size'], template: '<span />' },
  UiStageStepper: {
    props: ['stages', 'currentId', 'failedId'],
    template:
      '<ol class="stepper-stub" :data-current="currentId" :data-failed="failedId"><li v-for="s in stages" :key="s.id" :data-id="s.id">{{ s.label }}</li></ol>',
  },
  UiProgress: {
    props: ['value', 'max', 'showPercent', 'ariaLabel'],
    template: '<div class="progress-stub" :data-value="value" :data-max="max" />',
  },
  UiButton: {
    props: ['variant', 'iconLeft'],
    emits: ['click'],
    template: '<button class="cancel-stub" @click="$emit(\'click\', $event)"><slot /></button>',
  },
}

const progress = {
  jobId: 'j',
  stage: 'parse',
  current: 3,
  total: 7,
  message: 'pointer.png を解析中',
}

function mountOverlay(p: unknown) {
  return mount(BulkImportProgressOverlay, { props: { progress: p }, global: { stubs } })
}

describe('BulkImportProgressOverlay', () => {
  it('renders the fixed overlay shell', () => {
    expect(mountOverlay(progress).find('.bulk-overlay').exists()).toBe(true)
  })

  it('passes scan/parse/extract stages with currentId = progress.stage', () => {
    const st = mountOverlay(progress).find('.stepper-stub')
    expect(st.attributes('data-current')).toBe('parse')
    expect(st.findAll('li').map((li) => li.attributes('data-id'))).toEqual([
      'scan',
      'parse',
      'extract',
    ])
  })

  it('feeds current/total to the progress bar', () => {
    const p = mountOverlay(progress).find('.progress-stub')
    expect(p.attributes('data-value')).toBe('3')
    expect(p.attributes('data-max')).toBe('7')
  })

  it('renders the live progress message', () => {
    expect(mountOverlay(progress).text()).toContain('pointer.png を解析中')
  })

  it('marks the failed stage when stage = error', () => {
    expect(
      mountOverlay({ ...progress, stage: 'error' })
        .find('.stepper-stub')
        .attributes('data-failed'),
    ).toBe('error')
  })

  it('does not mark a failed stage in a normal stage', () => {
    expect(mountOverlay(progress).find('.stepper-stub').attributes('data-failed')).toBeUndefined()
  })

  it('defaults currentId to scan when progress is null', () => {
    expect(mountOverlay(null).find('.stepper-stub').attributes('data-current')).toBe('scan')
  })

  it('emits cancel when the cancel button is clicked', async () => {
    const w = mountOverlay(progress)
    await w.find('.cancel-stub').trigger('click')
    expect(w.emitted('cancel')).toHaveLength(1)
  })
})
