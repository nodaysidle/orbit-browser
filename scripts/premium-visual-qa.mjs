import { chromium } from 'playwright'
import { mkdir } from 'node:fs/promises'
import { resolve } from 'node:path'

const baseUrl = process.env.ORBIT_VISUAL_QA_URL || 'http://127.0.0.1:4178/'
const outDir = resolve(process.env.ORBIT_VISUAL_QA_OUT || 'artifacts/premium-visual-qa')

await mkdir(outDir, { recursive: true })

const browser = await chromium.launch({ headless: true })
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, deviceScaleFactor: 1 })
const consoleMessages = []
page.on('console', message => consoleMessages.push(`${message.type()}: ${message.text()}`))
page.on('pageerror', error => consoleMessages.push(`pageerror: ${error.message}`))

async function captureTheme(theme) {
  const url = new URL(baseUrl)
  url.searchParams.set('orbit-visual-qa', theme)
  await page.goto(url.toString(), { waitUntil: 'networkidle' })
  try {
    await page.waitForSelector('[data-tab-id="qa-home"]', { timeout: 5000 })
  } catch (error) {
    console.error(JSON.stringify({ theme, url: url.toString(), consoleMessages, html: (await page.content()).slice(0, 1200) }, null, 2))
    throw error
  }
  await page.locator('#addressInput').focus()

  const metrics = await page.evaluate(async () => {
    const frames = []
    let previous = performance.now()
    for (let i = 0; i < 90; i += 1) {
      await new Promise(resolveFrame => requestAnimationFrame(now => {
        frames.push(now - previous)
        previous = now
        resolveFrame()
      }))
    }
    const sorted = [...frames].sort((a, b) => a - b)
    const p95 = sorted[Math.floor(sorted.length * 0.95)] || 0
    const max = sorted.at(-1) || 0
    const tabCount = document.querySelectorAll('[data-tab-id]').length
    const activeTab = document.querySelector('[role="tab"][aria-selected="true"]')?.textContent?.trim() || ''
    const focusVisible = document.activeElement?.id === 'addressInput'
    const overflowX = document.documentElement.scrollWidth > window.innerWidth
    const overflowY = document.documentElement.scrollHeight > window.innerHeight
    const chromeHeight = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--chrome-height')) || 124
    const titlebarRect = document.querySelector('.titlebar')?.getBoundingClientRect()
    const settingsModal = document.querySelector('#settingsModal')
    settingsModal.classList.remove('hidden')
    const settingsRect = settingsModal.getBoundingClientRect()
    const settingsPanelRect = document.querySelector('.settings-panel')?.getBoundingClientRect()
    const settingsKeepsChromeVisible = Boolean(
      titlebarRect && settingsRect.top >= chromeHeight - 1 && settingsPanelRect && settingsPanelRect.top >= chromeHeight
    )
    return {
      theme: document.documentElement.dataset.theme,
      tabCount,
      activeTab,
      focusVisible,
      overflowX,
      overflowY,
      p95FrameMs: Number(p95.toFixed(2)),
      maxFrameMs: Number(max.toFixed(2)),
      settingsKeepsChromeVisible,
      titlebarBottom: Number(titlebarRect?.bottom?.toFixed(1) || 0),
      settingsTop: Number(settingsRect.top.toFixed(1)),
      settingsPanelTop: Number(settingsPanelRect?.top?.toFixed(1) || 0),
    }
  })

  if (metrics.overflowX || metrics.overflowY || !metrics.focusVisible || !metrics.settingsKeepsChromeVisible) {
    throw new Error(`visual QA failed for ${theme}: ${JSON.stringify(metrics)}`)
  }

  const screenshot = `${outDir}/orbit-${theme}-1440x1000.png`
  await page.screenshot({ path: screenshot, fullPage: false })
  return { screenshot, metrics }
}

const dark = await captureTheme('dark')
const light = await captureTheme('light')

await browser.close()

console.log(JSON.stringify({ baseUrl, outDir, dark, light }, null, 2))
