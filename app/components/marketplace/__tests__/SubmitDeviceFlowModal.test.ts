import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'

const invokeMock = vi.fn()
vi.mock('~/composables/useTauri', () => ({
  invokeTauri: (...a: unknown[]) => invokeMock(...a),
}))

// Stub UiIcon (it lazy-loads icon modules which is noisy in tests)
vi.mock('~/components/icons/UiIcon.vue', () => ({
  default: { template: '<span data-test="icon" />' },
}))

import SubmitDeviceFlowModal from '../SubmitDeviceFlowModal.vue'

describe('SubmitDeviceFlowModal', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    // Make clipboard write a no-op
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
      configurable: true,
      writable: true,
    })
  })

  it('shows user_code when start_device_flow succeeds', async () => {
    invokeMock.mockResolvedValueOnce({
      userCode: 'WDJB-MJHT',
      verificationUri: 'https://github.com/login/device',
      expiresIn: 900,
      interval: 5,
    })
    const w = mount(SubmitDeviceFlowModal, {
      props: { open: true },
      attachTo: document.body,
    })
    await flushPromises()
    expect(document.body.textContent).toContain('WDJB-MJHT')
    w.unmount()
  })

  it('does nothing when open is false', async () => {
    const w = mount(SubmitDeviceFlowModal, {
      props: { open: false },
      attachTo: document.body,
    })
    await flushPromises()
    expect(invokeMock).not.toHaveBeenCalled()
    expect(document.body.textContent).not.toContain('WDJB-MJHT')
    w.unmount()
  })

  it('calls open_url IPC when "Open GitHub" is clicked', async () => {
    invokeMock.mockResolvedValueOnce({
      userCode: 'X',
      verificationUri: 'https://github.com/login/device',
      expiresIn: 900,
      interval: 5,
    })
    const w = mount(SubmitDeviceFlowModal, {
      props: { open: true },
      attachTo: document.body,
    })
    await flushPromises()

    invokeMock.mockResolvedValueOnce(undefined) // for open_url
    const buttons = Array.from(document.body.querySelectorAll('button'))
    const openBtn = buttons.find(
      (b) => b.textContent?.includes('GitHub') && !b.textContent?.includes('連携'),
    )
    expect(openBtn).toBeTruthy()
    openBtn!.click()
    await flushPromises()
    expect(invokeMock).toHaveBeenCalledWith('open_url', {
      url: 'https://github.com/login/device',
    })
    w.unmount()
  })
})
