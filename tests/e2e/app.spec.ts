/**
 * E2E App tests
 */

import { test, expect } from '@playwright/test'
import { _electron as electron } from 'playwright'

test.describe('Orbit Browser', () => {
  let electronApp: Awaited<ReturnType<typeof electron.launch>>
  
  test.beforeAll(async () => {
    electronApp = await electron.launch({
      args: ['out/main/index.js'],
      cwd: process.cwd()
    })
  })
  
  test.afterAll(async () => {
    await electronApp.close()
  })
  
  test('app launches with correct title', async () => {
    const window = await electronApp.firstWindow()
    const title = await window.title()
    expect(title).toContain('Orbit')
  })
  
  test('window has correct dimensions', async () => {
    const window = await electronApp.firstWindow()
    const bounds = await window.evaluate(() => {
      const { width, height } = window.screen
      return { width, height }
    })
    expect(bounds.width).toBeGreaterThan(0)
    expect(bounds.height).toBeGreaterThan(0)
  })
  
  test('can take screenshot', async () => {
    const window = await electronApp.firstWindow()
    await window.screenshot({ path: 'test-results/initial.png' })
  })
})
