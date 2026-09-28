import { $, expect } from '@wdio/globals'

describe('launch', () => {
  it('renders the library page with sidebar nav', async () => {
    await $('[data-testid="nav-library"]').waitForDisplayed()
    await expect($('[data-testid="nav-settings"]')).toBeDisplayed()
    // main 領域が空でない (LibraryEmptyState か ThemeCard のどちらかが出る)
    const main = await $('main')
    await expect(main).toBeDisplayed()
    expect((await main.getText()).length).toBeGreaterThan(0)
  })
})
