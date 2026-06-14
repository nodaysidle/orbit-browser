import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getVersion } from '@tauri-apps/api/app'

import { bindEvents } from './events.js'
import { el, icon } from './utils/dom.js'
import {
  renderBookmarksList,
  renderHistoryList,
  renderRecentPages,
  renderShortcutEditor,
  renderShortcutGrid,
  renderTabs as renderTabList,
} from './utils/render.js'
import {
  formatError,
  getNavigationTitle,
  isEditableTarget,
  nextTheme,
  normalizeNavigationInput,
  normalizeSearchEngine,
  normalizeTheme,
  restoreNavigationSnapshot,
  themeIcon,
} from './utils/ui.js'

const state = {
  tabs: new Map(),
  activeId: null,
  historySearchTimer: null,
  loadTimeouts: new Map(),
  tabsRenderFrame: 0,
  resizeFrame: 0,
  theme: 'dark',
  resolvedTheme: 'dark',
  searchEngine: 'duckduckgo',
  startupBehavior: 'restore',
  shortcuts: [],
  errorPages: new Map(),
  systemThemeQuery: null,
  overlayHeight: 0,
  chromeHeightCache: null,
  unlisteners: [],
  zoomMemory: new Map(), // origin -> zoom level (e.g. "https://example.com" -> 1.2)
}

const win = getCurrentWindow()
const $ = id => document.getElementById(id)
const MAX_OVERLAY_HEIGHT = 680
const STARTUP_BEHAVIORS = new Set(['restore', 'new_tab'])
const DEFAULT_SHORTCUTS = [
  { title: 'NODAYSIDLE GitLab', url: 'https://gitlab.com/NODAYSIDLE' },
  { title: 'YouTube', url: 'https://youtube.com' },
  { title: 'Product Hunt', url: 'https://producthunt.com' },
  { title: 'Telegram Web', url: 'https://web.telegram.org' },
]
const EDITABLE_SHORTCUT_KEYS = new Set(['l', 'r', 'f', 'g', '[', ']', '.', 'h'])
const SHORTCUT_ACTIONS = new Map([
  ['t', { type: 'new-tab' }],
  ['shift+h', { type: 'home' }],
  ['w', { type: 'close-tab' }],
  ['l', { type: 'focus-address' }],
  ['r', { type: 'reload' }],
  ['f', { type: 'find' }],
  ['g', { type: 'find-next' }],
  ['=', { type: 'zoom-in' }],
  ['+', { type: 'zoom-in' }],
  ['-', { type: 'zoom-out' }],
  ['0', { type: 'reset-zoom' }],
  ['shift+r', { type: 'toggle-reader' }],
  ['.', { type: 'stop' }],
  ['[', { type: 'back' }],
  [']', { type: 'forward' }],
  ['shift+[', { type: 'previous-tab' }],
  ['shift+]', { type: 'next-tab' }],
])

function logError(...args) {
  console.error(...args)
}

function activeTab() { return state.tabs.get(state.activeId) || null }
function isWebUrl(url = '') {
  const lower = String(url).toLowerCase()
  return lower.startsWith('http://') || lower.startsWith('https://')
}

function normalizeStartupBehavior(value = '') {
  return STARTUP_BEHAVIORS.has(value) ? value : 'restore'
}

function parseShortcuts(value) {
  if (!value) return [...DEFAULT_SHORTCUTS]
  try {
    const parsed = JSON.parse(value)
    if (!Array.isArray(parsed)) return [...DEFAULT_SHORTCUTS]
    const shortcuts = parsed
      .map(shortcut => ({
        title: String(shortcut?.title || '').trim(),
        url: String(shortcut?.url || '').trim(),
      }))
      .filter(shortcut => shortcut.title && isWebUrl(normalizeNavigationInput(shortcut.url, state.searchEngine)))
      .slice(0, 4)
    return shortcuts.length ? shortcuts : [...DEFAULT_SHORTCUTS]
  } catch (error) {
    logError('shortcut settings parse failed', error)
    return [...DEFAULT_SHORTCUTS]
  }
}

function shortcutSettingsValue() {
  return JSON.stringify(state.shortcuts.map(shortcut => ({
    title: shortcut.title,
    url: normalizeNavigationInput(shortcut.url, state.searchEngine),
  })))
}

function absorb(promise, fallbackMessage = 'Action failed') {
  promise.catch(error => {
    if (error) logError('Action failed', error)
    if (!error?.orbitHandled) showToast(formatError(error, fallbackMessage))
  })
}

async function command(name, payload = {}, fallbackMessage = 'Action failed') {
  try {
    return await invoke(name, payload)
  } catch (error) {
    if (!isExpectedCommandError(error)) {
      logError(`${name} failed`, error)
    }
    const handled = new Error(formatError(error, fallbackMessage), { cause: error })
    handled.orbitHandled = true
    showToast(handled.message)
    throw handled
  }
}

function isExpectedCommandError(error) {
  return typeof error === 'string' && error.startsWith('Blocked by Orbit:')
}

function showToast(message, tone = 'error') {
  const region = $('toastRegion')
  if (!region || !message) return

  const toast = el('div', { className: `toast toast-${tone}`, text: message })
  region.append(toast)
  requestAnimationFrame(() => toast.classList.add('visible'))
  window.setTimeout(() => {
    toast.classList.remove('visible')
    window.setTimeout(() => toast.remove(), 190)
  }, 3200)
}

function showConfirmToast({ message, confirmLabel = 'Download', cancelLabel = 'Cancel', onConfirm, onCancel }) {
  const region = $('toastRegion')
  if (!region || !message) return

  const toast = el('div', { className: 'toast toast-info toast-actions-wrap' })
  const content = el('div', { className: 'toast-message', text: message })
  const confirm = el('button', { className: 'toast-action toast-action-primary', type: 'button', text: confirmLabel })
  const cancel = el('button', { className: 'toast-action', type: 'button', text: cancelLabel })
  const actions = el('div', { className: 'toast-actions' }, [confirm, cancel])
  toast.replaceChildren(content, actions)
  region.append(toast)
  requestAnimationFrame(() => toast.classList.add('visible'))

  const close = () => {
    toast.classList.remove('visible')
    window.setTimeout(() => toast.remove(), 190)
  }

  confirm.addEventListener('click', () => {
    try { onConfirm?.() } finally { close() }
  })
  cancel.addEventListener('click', () => {
    try { onCancel?.() } finally { close() }
  })

  window.setTimeout(() => {
    if (toast.isConnected) close()
  }, 12000)
}

function setIconButton(button, name, size = 18) { button.replaceChildren(icon(name, size)) }

function renderTabs() {
  renderTabList({ container: $('tabsContainer'), tabs: [...state.tabs.values()], activeId: state.activeId, tabCount: $('tabCount') })
  updateChromeProgress()
  updateTabOverflowIndicators()
}

function updateTabOverflowIndicators() {
  const container = $('tabsContainer')
  if (!container) return
  const hasLeft = container.scrollLeft > 4
  const hasRight = container.scrollLeft + container.clientWidth < container.scrollWidth - 4
  container.classList.toggle('has-overflow-left', hasLeft)
  container.classList.toggle('has-overflow-right', hasRight)
}

function setupTabOverflowIndicators() {
  const container = $('tabsContainer')
  if (!container) return
  const handler = () => updateTabOverflowIndicators()
  container.addEventListener('scroll', handler, { passive: true })
  // Also react to window resizes and tab count changes (renderTabs already calls the updater)
  window.addEventListener('resize', handler, { passive: true })
  // Initial state after first paint
  requestAnimationFrame(handler)
}

function queueRenderTabs() {
  if (state.tabsRenderFrame) return
  state.tabsRenderFrame = requestAnimationFrame(() => {
    state.tabsRenderFrame = 0
    renderTabs()
  })
}

function renderHistory(entries = []) { renderHistoryList($('historyList'), entries) }

function renderBookmarks(bookmarks = []) { renderBookmarksList($('bookmarksList'), bookmarks) }

function updateBookmarkIcon(bookmarked = false) { $('btnBookmark').classList.toggle('bookmarked', bookmarked) }

function renderShortcuts() {
  renderShortcutGrid($('shortcutsRow'), state.shortcuts)
}

function updateChromeProgress() {
  const progress = $('chromeProgress')
  if (!progress) return
  progress.classList.toggle('active', [...state.tabs.values()].some(tab => tab.loading))
}

function getSystemTheme() {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function resolvedThemeFor(value) {
  const theme = normalizeTheme(value)
  return theme === 'auto' ? getSystemTheme() : theme
}

function themeButtonLabel() {
  if (state.theme === 'auto') return `Theme follows system (${state.resolvedTheme}). Switch to dark mode`
  if (state.theme === 'dark') return 'Theme set to dark. Switch to light mode'
  return 'Theme set to light. Switch to auto mode'
}

function applyTheme(value) {
  state.theme = normalizeTheme(value)
  state.resolvedTheme = resolvedThemeFor(state.theme)
  document.documentElement.dataset.theme = state.resolvedTheme
  document.documentElement.dataset.themePreference = state.theme
  setIconButton($('btnTheme'), themeIcon(state.theme), 18)
  $('btnTheme').setAttribute('aria-label', themeButtonLabel())
  $('btnTheme').title = themeButtonLabel()
}

function setupSystemThemeListener() {
  if (!window.matchMedia) return
  const query = window.matchMedia('(prefers-color-scheme: dark)')
  state.systemThemeQuery = query
  const onChange = () => {
    if (state.theme === 'auto') applyTheme('auto')
  }
  query.addEventListener?.('change', onChange)
  state.unlisteners.push(() => query.removeEventListener?.('change', onChange))
}

async function syncBrowserView() {
  const width = Math.max(1, window.innerWidth)
  const height = Math.max(1, window.innerHeight)
  try {
    await invoke('sync_browser_view', { width, height })
  } catch (error) {
    logError('sync_browser_view failed', error)
  }
}

function chromeHeight() {
  if (state.chromeHeightCache !== null) return state.chromeHeightCache
  const value = getComputedStyle(document.documentElement).getPropertyValue('--chrome-height').trim()
  const parsed = Number.parseFloat(value)
  state.chromeHeightCache = Number.isFinite(parsed) ? parsed : 124
  return state.chromeHeightCache
}

function activeOverlayElement() {
  const history = $('historyPanel')
  if (history && !history.classList.contains('hidden')) return history
  const bookmarks = $('bookmarksPanel')
  if (bookmarks && !bookmarks.classList.contains('hidden')) return bookmarks
  const menu = $('menuDropdown')
  if (menu && !menu.classList.contains('hidden')) return menu
  const findBar = $('findBar')
  if (findBar && !findBar.classList.contains('hidden')) return findBar
  return null
}

function computeOverlayHeight() {
  const overlay = activeOverlayElement()
  if (!overlay) return 0
  const chrome = chromeHeight()
  const rect = overlay.getBoundingClientRect()
  const next = Math.ceil(rect.bottom - chrome + 8)
  return Number.isFinite(next) ? Math.max(0, Math.min(MAX_OVERLAY_HEIGHT, next)) : 0
}

async function syncOverlayHeight() {
  const next = computeOverlayHeight()
  if (next === state.overlayHeight) return
  state.overlayHeight = next
  try {
    await invoke('set_overlay_height', { height: next })
  } catch (error) {
    logError('set_overlay_height failed', error)
  }
}

function updateOverlay() { absorb(syncOverlayHeight()) }

function queueBrowserViewSync() {
  if (state.resizeFrame) cancelAnimationFrame(state.resizeFrame)
  state.resizeFrame = requestAnimationFrame(() => {
    state.resizeFrame = 0
    absorb(syncBrowserView())
  })
}

function clearLoadTimeout(tabId) {
  if (!tabId) return
  const timer = state.loadTimeouts.get(tabId)
  if (timer) {
    window.clearTimeout(timer)
    state.loadTimeouts.delete(tabId)
  }
}

function scheduleLoadTimeout(tabId) {
  if (!tabId) return
  const tab = state.tabs.get(tabId)
  if (!tab?.loading) {
    clearLoadTimeout(tabId)
    return
  }
  clearLoadTimeout(tabId)
  const token = (tab.loadingToken || 0) + 1
  tab.loadingToken = token
  const timeout = window.setTimeout(() => {
    const current = state.tabs.get(tabId)
    if (!current?.loading || current.loadingToken !== token) return
    current.loading = false
    queueRenderTabs()
    if (tabId === state.activeId) updateNav()
    setTabError(tabId, {
      title: 'Page load timed out',
      message: 'Orbit did not receive a finished load event. The server may be offline or unreachable.',
      url: current.url,
    })
    showToast('Page is taking longer than expected. You can stop or reload.', 'info')
  }, 15000)
  state.loadTimeouts.set(tabId, timeout)
}

async function refreshBookmarkIcon() {
  const tab = activeTab()
  if (!isWebUrl(tab?.url)) {
    updateBookmarkIcon(false)
    return
  }

  const tabId = tab.id
  const url = tab.url
  try {
    const marked = await invoke('is_bookmarked', { url })
    if (state.activeId === tabId && activeTab()?.url === url) updateBookmarkIcon(marked)
  } catch (error) {
    logError('is_bookmarked failed', error)
    if (state.activeId === tabId && activeTab()?.url === url) updateBookmarkIcon(false)
  }
}

function activeError() {
  return state.errorPages.get(state.activeId) || null
}

function setTabError(tabId, details) {
  if (!tabId) return
  state.errorPages.set(tabId, {
    title: 'Orbit could not open this page',
    message: 'The navigation failed before a page could load.',
    url: '',
    ...details,
  })
  if (tabId === state.activeId) {
    $('newTabPage')?.classList.remove('hidden')
    renderErrorState()
  }
}

function clearTabError(tabId) {
  if (!tabId) return
  state.errorPages.delete(tabId)
  if (tabId === state.activeId) renderErrorState()
}

function renderErrorState() {
  const error = activeError()
  const home = $('homeContent')
  const page = $('errorPage')
  if (!home || !page) return
  home.classList.toggle('hidden', Boolean(error))
  page.classList.toggle('hidden', !error)
  if (!error) return
  $('errorTitle').textContent = error.title
  $('errorMessage').textContent = error.message
  $('errorUrl').textContent = error.url || ''
}

function updateNav() {
  const tab = activeTab()
  const url = tab?.url || ''
  const hasPage = isWebUrl(url)
  const error = activeError()
  const wrap = $('addressWrap')

  $('addressInput').value = hasPage || error ? (error?.url || url) : ''
  $('btnHome').disabled = !state.activeId || !hasPage
  $('btnBack').disabled = !tab?.can_go_back
  $('btnForward').disabled = !tab?.can_go_forward
  $('btnReload').disabled = !hasPage
  $('btnStop').disabled = !hasPage || !tab?.loading
  $('btnBookmark').disabled = !hasPage
  $('btnReload').classList.toggle('loading', Boolean(tab?.loading))

  // Persistent security pill + scheme distinction (Slice 1)
  const lower = String(url).toLowerCase()
  const isHttps = lower.startsWith('https://')
  const isHttp = lower.startsWith('http://')
  const schemeEl = $('addressScheme')
  if (wrap) {
    if (hasPage) {
      wrap.setAttribute('data-scheme', isHttps ? 'https' : 'http')
      if (schemeEl) schemeEl.textContent = isHttps ? 'https' : 'http'
    } else {
      wrap.setAttribute('data-scheme', 'search')
      if (schemeEl) schemeEl.textContent = ''
    }
  }
  $('lockIcon').style.color = isHttps ? 'var(--teal)' : 'var(--quiet)'

  wrap?.setAttribute('data-url-preview', hasPage ? url : '')
  $('newTabPage').classList.toggle('hidden', hasPage && !error)
  renderErrorState()
  updateChromeProgress()
  absorb(refreshBookmarkIcon())
}

function cleanUrlForCopy(rawUrl) {
  try {
    const url = new URL(rawUrl)
    const paramsToRemove = [
      'utm_source', 'utm_medium', 'utm_campaign', 'utm_term', 'utm_content',
      'fbclid', 'gclid', 'dclid', 'msclkid', '_ga', 'ref', 'mc_cid', 'mc_eid'
    ]
    paramsToRemove.forEach(key => url.searchParams.delete(key))

    // Also remove common tracking params that start with certain prefixes
    for (const [key] of url.searchParams) {
      if (key.startsWith('utm_') || key.startsWith('mc_')) {
        url.searchParams.delete(key)
      }
    }

    // Remove hash if it's just tracking (common on some sites)
    if (url.hash && /^#?utm_|^#?ref=/.test(url.hash)) {
      url.hash = ''
    }

    return url.toString()
  } catch {
    return rawUrl
  }
}

function copyCurrentAddress() {
  const tab = activeTab()
  const rawUrl = tab?.url
  if (!rawUrl) return

  const cleanUrl = cleanUrlForCopy(rawUrl)

  navigator.clipboard.writeText(cleanUrl).then(() => {
    showToast('Clean link copied')
  }).catch(() => {
    const input = $('addressInput')
    input.value = cleanUrl
    input.select()
    document.execCommand('copy')
    showToast('Clean link copied')
  })
}

function deleteShortcutAt(index) {
  if (index < 0 || index >= state.shortcuts.length) return
  state.shortcuts.splice(index, 1)
  renderShortcuts()
  // Persist using the same settings path as the editor
  absorb(saveSetting('shortcuts', JSON.stringify(state.shortcuts), 'Could not save shortcut deletion'))
}

// --- Zoom Memory (Per-origin) ---
function getOrigin(url) {
  try {
    return new URL(url).origin
  } catch {
    return null
  }
}

async function loadZoomMemory() {
  try {
    const raw = await getSetting('zoom_memory')
    if (raw) {
      const parsed = JSON.parse(raw)
      state.zoomMemory = new Map(Object.entries(parsed))
    }
  } catch (e) {
    logError('Failed to load zoom memory', e)
  }
}

async function saveZoomMemory() {
  try {
    const obj = Object.fromEntries(state.zoomMemory)
    await saveSetting('zoom_memory', JSON.stringify(obj), 'Could not save zoom memory')
  } catch (e) {
    logError('Failed to save zoom memory', e)
  }
}

function saveCurrentZoom(forcedValue = null) {
  const tab = activeTab()
  if (!tab?.url) return
  const origin = getOrigin(tab.url)
  if (!origin) return

  // For simplicity in this implementation, we store a coarse value.
  // In a fuller version we would read the actual current zoom from the webview.
  const value = forcedValue !== null ? forcedValue : 1.0
  state.zoomMemory.set(origin, value)
  absorb(saveZoomMemory())
}

function applyZoomToActiveTab() {
  const tab = activeTab()
  if (!tab?.url) return
  const origin = getOrigin(tab.url)
  if (!origin) return

  const savedZoom = state.zoomMemory.get(origin)
  if (!savedZoom || savedZoom === 1.0) return

  // Apply saved zoom using direct eval (now supported via eval_on_tab)
  const factor = savedZoom.toFixed(1)
  absorb(command('eval_on_tab', { tabId: state.activeId, script: `document.body.style.zoom = '${factor}';` }))
}

// --- Reader Mode (Local only) ---
let readerModeActive = false

function toggleReaderMode() {
  const tab = activeTab()
  if (!tab) return

  readerModeActive = !readerModeActive

  const readerCSS = `
    :root { --reader-max-width: 680px; }
    body {
      max-width: var(--reader-max-width) !important;
      margin: 40px auto !important;
      padding: 0 20px !important;
      font-family: Georgia, 'Times New Roman', serif !important;
      font-size: 18px !important;
      line-height: 1.7 !important;
      color: #222 !important;
      background: #f9f5eb !important;
    }
    img, video, iframe { max-width: 100%; height: auto; }
    pre, code { font-family: ui-monospace, monospace; }
    h1, h2, h3 { font-family: -apple-system, system-ui, sans-serif; }
  `.replace(/\s+/g, ' ')

  if (readerModeActive) {
    absorb(command('eval_on_tab', { tabId: state.activeId, script: `
      (function() {
        const style = document.createElement('style');
        style.id = 'orbit-reader-mode';
        style.textContent = \`${readerCSS}\`;
        document.head.appendChild(style);
      })();
    ` }))
    showToast('Reader mode on')
  } else {
    absorb(command('eval_on_tab', { tabId: state.activeId, script: `
      const style = document.getElementById('orbit-reader-mode');
      if (style) style.remove();
    ` }))
    showToast('Reader mode off')
  }
}

function closeAllPanels() {
  $('menuDropdown').classList.add('hidden')
  $('btnMenu').setAttribute('aria-expanded', 'false')
  $('historyPanel').classList.add('hidden')
  $('bookmarksPanel').classList.add('hidden')
  updateOverlay()
}

async function createTab(url = '') {
  await syncBrowserView()
  const tab = await command('create_tab', { url, makeActive: true }, 'Could not create a new tab')
  state.tabs.set(tab.id, tab)
  state.activeId = tab.id
  clearTabError(tab.id)
  renderTabs()
  updateNav()
  if (!url) $('addressInput').focus()
  absorb(loadRecentPages())
}

async function switchTab(tabId) {
  if (!tabId || tabId === state.activeId) return
  closeAllPanels()
  await syncBrowserView()
  await command('switch_tab', { tabId }, 'Could not switch tabs')
  state.activeId = tabId
  renderTabs()
  updateNav()
  absorb(loadRecentPages())
}

async function closeTab(tabId) {
  if (!tabId) return
  clearLoadTimeout(tabId)
  const newActiveId = await command('close_tab', { tabId }, 'Could not close the tab')
  state.tabs.delete(tabId)
  state.errorPages.delete(tabId)

  if (state.tabs.size === 0) {
    await createTab()
    return
  }

  if (state.activeId === tabId) {
    state.activeId = newActiveId || [...state.tabs.keys()][0] || null
    if (state.activeId) {
      await command('switch_tab', { tabId: state.activeId }, 'Could not switch tabs')
    }
  }

  renderTabs()
  updateNav()
}

async function navigate(rawUrl) {
  if (!rawUrl?.trim() || !state.activeId) return
  const tabId = state.activeId
  const before = { ...activeTab() }
  const targetUrl = normalizeNavigationInput(rawUrl, state.searchEngine)
  closeAllPanels()
  previewNavigation(rawUrl, targetUrl)
  await syncBrowserView()
  try {
    await command('navigate_tab', { tabId, url: targetUrl }, 'Could not open that page')
  } catch (error) {
    restoreNavigation(tabId, before)
    setTabError(tabId, {
      title: isExpectedCommandError(error.message) ? 'Blocked by Orbit' : 'Orbit could not open this page',
      message: formatError(error, 'The navigation failed before a page could load.'),
      url: targetUrl,
    })
  }
}

function previewNavigation(rawUrl, targetUrl = normalizeNavigationInput(rawUrl, state.searchEngine)) {
  const tab = activeTab()
  if (!tab) return
  clearTabError(tab.id)
  tab.url = targetUrl
  tab.title = getNavigationTitle(rawUrl)
  tab.loading = true
  scheduleLoadTimeout(tab.id)
  updateBookmarkIcon(false)
  renderTabs()
  updateNav()
}

function restoreNavigation(tabId, snapshot) {
  clearLoadTimeout(tabId)
  state.tabs = restoreNavigationSnapshot(state.tabs, tabId, snapshot)
  renderTabs()
  if (state.activeId === tabId) updateNav()
}

async function openHistoryPanel() {
  closeAllPanels()
  $('historySearch').value = ''
  const entries = await command('get_history', { limit: 50, offset: 0 }, 'Could not load history')
  renderHistory(entries)
  $('historyPanel').classList.remove('hidden')
  updateOverlay()
}

async function openBookmarksPanel() {
  closeAllPanels()
  const bookmarks = await command('get_bookmarks', {}, 'Could not load bookmarks')
  renderBookmarks(bookmarks)
  $('bookmarksPanel').classList.remove('hidden')
  updateOverlay()
}

async function toggleBookmark() {
  const tab = activeTab()
  if (!isWebUrl(tab?.url)) return

  const marked = await command('is_bookmarked', { url: tab.url }, 'Could not check bookmark status')
  if (marked) {
    const bookmarks = await command('get_bookmarks', {}, 'Could not load bookmarks')
    const bookmark = bookmarks.find(entry => entry.url === tab.url)
    if (bookmark) await command('delete_bookmark', { id: bookmark.id }, 'Could not remove bookmark')
    updateBookmarkIcon(false)
    showToast('Bookmark removed', 'success')
    return
  }

  await command('add_bookmark', { url: tab.url, title: tab.title || tab.url }, 'Could not save bookmark')
  updateBookmarkIcon(true)
  showToast('Bookmark saved', 'success')
}

async function handleDownload(url) {
  const filename = url.split('/').pop()?.split('?')[0] || 'file'
  showConfirmToast({
    message: `Download ${filename}?`,
    onConfirm: () => {
      absorb((async () => {
        try {
          await invoke('download_file', { url })
        } catch (error) {
          logError('download_file failed', error)
          showToast(formatError(error, 'Download failed'))
        }
      })())
    },
    onCancel: () => showToast('Download canceled', 'info'),
  })
}

export function applyLoadedTabToMap(tabs, payload) {
  if (!payload?.id || !tabs.has(payload.id)) return tabs
  const next = new Map(tabs)
  const existing = next.get(payload.id) || {}
  next.set(payload.id, { ...existing, ...payload, loading: false })
  return next
}

export function applyProgressToMap(tabs, payload) {
  if (!payload?.id || !tabs.has(payload.id)) return tabs
  const next = new Map(tabs)
  const existing = next.get(payload.id)
  next.set(payload.id, {
    ...existing,
    loading: true,
    url: payload.url || existing.url,
    title: payload.title || getNavigationTitle(payload.url) || existing.title,
  })
  return next
}

async function setupListeners() {
  const progress = ({ payload }) => applyTabProgress(payload)
  const loaded = ({ payload }) => {
    clearLoadTimeout(payload.id)
    state.tabs = applyLoadedTabToMap(state.tabs, payload)
    clearTabError(payload.id)
    queueRenderTabs()
    if (payload.id === state.activeId) updateNav()
    absorb(loadRecentPages())
  }
  const blocked = ({ payload }) => {
    const tab = payload?.tab
    if (tab?.id) {
      clearLoadTimeout(tab.id)
      state.tabs.set(tab.id, { ...tab, loading: false })
      setTabError(tab.id, {
        title: 'Blocked by Orbit',
        message: `Orbit blocked navigation to ${getNavigationTitle(payload?.blockedUrl || '')}.`,
        url: payload?.blockedUrl || tab.url,
      })
      queueRenderTabs()
      if (tab.id === state.activeId) updateNav()
    }
    showToast(`Blocked ${getNavigationTitle(payload?.blockedUrl || '')}`)
  }
  const favicon = ({ payload }) => {
    const tab = state.tabs.get(payload.id)
    if (tab) {
      // Payload contains "standard_url google_url" — split for fallback chain
      const urls = (payload.faviconUrl || '').split(' ')
      tab.favicon_url = urls[0] || null
      tab.favicon_fallback = urls[1] || null
      queueRenderTabs()
    }
  }
  const downloadDetected = ({ payload }) => {
    handleDownload(payload.url)
  }
  const downloadStarted = ({ payload }) => {
    showToast(`Downloading ${payload.filename}…`, 'info')
  }
  const downloadComplete = ({ payload }) => {
    showToast(`Downloaded ${payload.filename} to Downloads`, 'success')
  }
  const newWindow = ({ payload }) => {
    const url = payload?.url || ''
    if (!url) return
    closeAllPanels()
    absorb(createTab(url))
  }
  state.unlisteners.push(
    await listen('orbit-shortcut', ({ payload }) => handleNativeShortcut(payload)),
    await listen('orbit-about', showAboutPanel),
    await listen('tab-navigating', progress),
    await listen('tab-loading', progress),
    await listen('tab-loaded', loaded),
    await listen('tab-blocked', blocked),
    await listen('tab-favicon', favicon),
    await listen('tab-new-window', newWindow),
    await listen('download-detected', downloadDetected),
    await listen('download-started', downloadStarted),
    await listen('download-complete', downloadComplete),
  )
}

function applyTabProgress(payload) {
  if (!state.tabs.has(payload?.id)) return
  state.tabs = applyProgressToMap(state.tabs, payload)
  const tab = state.tabs.get(payload.id)
  clearTabError(payload.id)
  scheduleLoadTimeout(payload.id)
  queueRenderTabs()
  if (payload.id === state.activeId) updateNav()
}

function handleNativeShortcut(action) {
  const handlers = {
    'new-tab': () => absorb(createTab()),
    'close-tab': () => absorb(closeTab(state.activeId)),
    'focus-address': () => $('addressInput').focus(),
    reload: () => absorb(command('reload_tab', { tabId: state.activeId }, 'Could not reload this page')),
    stop: () => absorb(command('stop_tab', { tabId: state.activeId }, 'Could not stop loading')),
    home: () => absorb(command('go_home_tab', { tabId: state.activeId }, 'Could not open home')),
    back: () => absorb(command('go_back', { tabId: state.activeId }, 'Could not go back')),
    forward: () => absorb(command('go_forward', { tabId: state.activeId }, 'Could not go forward')),
    find: () => openFindBar(),
    'zoom-in': () => { absorb(command('zoom_tab', { tabId: state.activeId, zoomIn: true })); saveCurrentZoom(); },
    'zoom-out': () => { absorb(command('zoom_tab', { tabId: state.activeId, zoomIn: false })); saveCurrentZoom(); },
    'actual-size': () => { absorb(command('reset_zoom', { tabId: state.activeId })); saveCurrentZoom(1.0); },
    'toggle-reader': () => toggleReaderMode(),
    'show-bookmarks': () => absorb(openBookmarksPanel()),
    'show-history': () => absorb(openHistoryPanel()),
    settings: () => openSettingsPanel(),
  }
  handlers[action]?.()
}

async function clearHistory() {
  await command('clear_history', {}, 'Could not clear history')
  renderHistory([])
  closeAllPanels()
  showToast('History cleared', 'success')
}

async function deleteBookmark(id) {
  await command('delete_bookmark', { id }, 'Could not delete bookmark')
  await openBookmarksPanel()
  updateNav()
}

function queueHistorySearch(query) {
  clearTimeout(state.historySearchTimer)
  state.historySearchTimer = window.setTimeout(async () => {
    try {
      const entries = query.length > 1
        ? await command('search_history', { query }, 'Could not search history')
        : await command('get_history', { limit: 50, offset: 0 }, 'Could not load history')
      renderHistory(entries)
    } catch (error) {
      logError('history search failed', error)
    }
  }, 160)
}

async function getSetting(key) {
  try {
    return await invoke('get_setting', { key })
  } catch (error) {
    logError(`get_setting ${key} failed`, error)
    return null
  }
}

async function saveSetting(key, value, fallbackMessage = 'Could not save setting') {
  await command('set_setting', { key, value }, fallbackMessage)
}

async function loadPreferences() {
  const [theme, searchEngine, startupBehavior, shortcuts] = await Promise.all([
    getSetting('theme'),
    getSetting('search_engine'),
    getSetting('startup_behavior'),
    getSetting('shortcuts'),
  ])
  applyTheme(normalizeTheme(theme))
  state.searchEngine = normalizeSearchEngine(searchEngine)
  state.startupBehavior = normalizeStartupBehavior(startupBehavior)
  state.shortcuts = parseShortcuts(shortcuts)
  syncSettingsControls()
  renderShortcuts()
}

async function setThemePreference(theme) {
  const previous = state.theme
  applyTheme(theme)
  try {
    await saveSetting('theme', state.theme, 'Could not save theme')
  } catch (error) {
    logError('set_setting theme failed', error)
    applyTheme(previous)
  }
}

async function toggleTheme() {
  await setThemePreference(nextTheme(state.theme))
}

async function changeSearchEngine(value) {
  const previous = state.searchEngine
  state.searchEngine = normalizeSearchEngine(value)
  syncSettingsControls()
  try {
    await saveSetting('search_engine', state.searchEngine, 'Could not save search engine')
  } catch (error) {
    logError('set_setting search_engine failed', error)
    state.searchEngine = previous
    syncSettingsControls()
  }
}

async function changeStartupBehavior(value) {
  const previous = state.startupBehavior
  state.startupBehavior = normalizeStartupBehavior(value)
  syncSettingsControls()
  try {
    await saveSetting('startup_behavior', state.startupBehavior, 'Could not save startup setting')
  } catch (error) {
    logError('set_setting startup_behavior failed', error)
    state.startupBehavior = previous
    syncSettingsControls()
  }
}

const MODAL_FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
let activeModal = null
let activeModalTrigger = null
let activeModalFocusHandler = null

function collectFocusables(modal) {
  return Array.from(modal.querySelectorAll(MODAL_FOCUSABLE_SELECTOR)).filter(
    element => element.tabIndex >= 0 && !element.hasAttribute('disabled')
  )
}

function bindModalFocusTrap(modal) {
  const focusables = collectFocusables(modal)
  if (!focusables.length) return
  const first = focusables[0]
  const last = focusables[focusables.length - 1]

  if (!focusables.includes(document.activeElement)) {
    first.focus()
  }

  activeModalFocusHandler = event => {
    if (event.key !== 'Tab' || focusables.length === 0) return
    if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault()
      first.focus()
    } else if (event.shiftKey && document.activeElement === first) {
      event.preventDefault()
      last.focus()
    }
  }

  modal.addEventListener('keydown', activeModalFocusHandler)
}

function closeModal(modal) {
  if (!modal) return
  modal.classList.add('hidden')
  if (activeModalFocusHandler) {
    modal.removeEventListener('keydown', activeModalFocusHandler)
  }
  activeModalFocusHandler = null
  activeModal = null
  if (activeModalTrigger instanceof HTMLElement) {
    activeModalTrigger.focus()
  }
  activeModalTrigger = null
}

function openModal(modal, fallbackSelector) {
  if (!modal) return
  activeModal = modal
  activeModalTrigger = document.activeElement instanceof HTMLElement ? document.activeElement : null
  modal.classList.remove('hidden')
  const fallback = fallbackSelector ? modal.querySelector(fallbackSelector) : null
  if (fallback instanceof HTMLElement) fallback.focus()
  bindModalFocusTrap(modal)
}

async function saveShortcutEdits() {
  const rows = [...document.querySelectorAll('[data-shortcut-index]')]
  const next = rows.map(row => {
    const title = row.querySelector('[data-shortcut-title]')?.value.trim() || ''
    const url = row.querySelector('[data-shortcut-url-input]')?.value.trim() || ''
    return { title, url: normalizeNavigationInput(url, state.searchEngine) }
  }).filter(shortcut => shortcut.title && isWebUrl(shortcut.url)).slice(0, 4)
  if (!next.length) {
    showToast('Add at least one shortcut title and URL.', 'info')
    return
  }
  const previous = state.shortcuts
  state.shortcuts = next
  renderShortcuts()
  syncSettingsControls()
  try {
    await saveSetting('shortcuts', shortcutSettingsValue(), 'Could not save shortcuts')
    showToast('Shortcuts saved', 'success')
  } catch (error) {
    logError('set_setting shortcuts failed', error)
    state.shortcuts = previous
    renderShortcuts()
    syncSettingsControls()
  }
}

function syncSettingsControls() {
  const theme = $('settingTheme')
  const searchEngine = $('settingSearchEngine')
  const startup = $('settingStartup')
  if (theme) theme.value = state.theme
  if (searchEngine) searchEngine.value = state.searchEngine
  if (startup) startup.value = state.startupBehavior
  if ($('shortcutEditor')) renderShortcutEditor($('shortcutEditor'), state.shortcuts)
}

function openSettingsPanel() {
  closeAllPanels()
  syncSettingsControls()
  openModal($('settingsModal'), '#settingsClose')
}

function closeSettingsPanel() {
  closeModal($('settingsModal'))
}

async function loadRecentPages() {
  const container = $('recentPages')
  if (!container) return
  try {
    const entries = await invoke('get_history', { limit: 6, offset: 0 })
    renderRecentPages(container, entries)
  } catch (error) {
    logError('get_history recent failed', error)
    renderRecentPages(container, [])
  }
}

function handleNewTabSearch(event) {
  event.preventDefault()
  const input = $('newTabSearchInput')
  const value = input?.value.trim()
  if (!value) return
  absorb(navigate(value))
  input.blur()
}

function retryErrorPage() {
  const error = activeError()
  if (!error?.url) return
  clearTabError(state.activeId)
  absorb(navigate(error.url))
}

function errorPageHome() {
  clearTabError(state.activeId)
  absorb(command('go_home_tab', { tabId: state.activeId }, 'Could not open home'))
  const tab = activeTab()
  if (tab) {
    tab.url = ''
    tab.title = 'New Tab'
    tab.loading = false
  }
  renderTabs()
  updateNav()
}

function handleAddressKey(event) {
  if (event.key === 'Enter') {
    absorb(navigate(event.target.value))
    event.target.blur()
  } else if (event.key === 'Escape') {
    updateNav()
    event.target.blur()
  }
}

function handleShortcut(event) {
  const intent = getShortcutIntent(event)
  if (!intent) return

  if (intent.type === 'escape') {
    if (!$('findBar').classList.contains('hidden')) {
      closeFindBar()
      return
    }
    closeAllPanels()
    return
  }

  event.preventDefault()
  runShortcutIntent(intent)
}

export function getShortcutIntent(event) {
  if (event.key === 'Escape') return { type: 'escape' }
  const mod = event.metaKey || event.ctrlKey
  if (!mod) return null

  const key = event.key.toLowerCase()
  if (isEditableTarget(event.target) && !EDITABLE_SHORTCUT_KEYS.has(key)) return null
  if (event.key >= '1' && event.key <= '9') {
    return { type: 'switch-tab-index', index: Number.parseInt(event.key, 10) - 1 }
  }
  return SHORTCUT_ACTIONS.get(`${event.shiftKey ? 'shift+' : ''}${key}`) || null
}

function runShortcutIntent(intent) {
  const handlers = {
    'new-tab': () => absorb(createTab()),
    home: () => absorb(command('go_home_tab', { tabId: state.activeId }, 'Could not open home')),
    'close-tab': () => absorb(closeTab(state.activeId)),
    'focus-address': () => $('addressInput').focus(),
    reload: () => absorb(command('reload_tab', { tabId: state.activeId }, 'Could not reload this page')),
    find: () => openFindBar(),
    'find-next': () => findNext(),
    'zoom-in': () => absorb(command('zoom_tab', { tabId: state.activeId, zoomIn: true })),
    'zoom-out': () => absorb(command('zoom_tab', { tabId: state.activeId, zoomIn: false })),
    'reset-zoom': () => absorb(command('reset_zoom', { tabId: state.activeId })),
    'toggle-reader': () => toggleReaderMode(),
    stop: () => absorb(command('stop_tab', { tabId: state.activeId }, 'Could not stop loading')),
    back: () => absorb(command('go_back', { tabId: state.activeId }, 'Could not go back')),
    forward: () => absorb(command('go_forward', { tabId: state.activeId }, 'Could not go forward')),
    'previous-tab': () => cycleTab(-1),
    'next-tab': () => cycleTab(1),
    'switch-tab-index': () => {
      const tab = [...state.tabs.values()][intent.index]
      if (tab) absorb(switchTab(tab.id))
    },
  }
  handlers[intent.type]?.()
}

function cycleTab(direction) {
  const tabIds = [...state.tabs.keys()]
  if (tabIds.length < 2) return
  const currentIdx = tabIds.indexOf(state.activeId)
  const newIdx = (currentIdx + direction + tabIds.length) % tabIds.length
  absorb(switchTab(tabIds[newIdx]))
}

// ── Find in Page ───────────────────────────────────────────────────────────────

function openFindBar() {
  $('findBar').classList.remove('hidden')
  focusFindInput({ select: true })
  updateOverlay()
}

function closeFindBar() {
  $('findBar').classList.add('hidden')
  absorb(command('find_in_page', { tabId: state.activeId, query: '', backwards: false }))
  updateOverlay()
}

function focusFindInput({ select = false } = {}) {
  const input = $('findInput')
  input.focus()
  if (select) input.select()
}

function findNext() {
  const query = $('findInput').value.trim()
  focusFindInput()
  if (!query) return
  absorb(command('find_in_page', { tabId: state.activeId, query, backwards: false }))
}

function findPrev() {
  const query = $('findInput').value.trim()
  focusFindInput()
  if (!query) return
  absorb(command('find_in_page', { tabId: state.activeId, query, backwards: true }))
}

function showAboutPanel() {
  openModal($('aboutModal'), '#aboutClose')
}

function closeAboutPanel() {
  closeModal($('aboutModal'))
}

function cleanupListeners() {
  clearTimeout(state.historySearchTimer)
  if (state.tabsRenderFrame) cancelAnimationFrame(state.tabsRenderFrame)
  if (state.resizeFrame) cancelAnimationFrame(state.resizeFrame)
  for (const timeoutId of state.loadTimeouts.values()) {
    clearTimeout(timeoutId)
  }
  state.loadTimeouts.clear()
  for (const unlisten of state.unlisteners) {
    try {
      const result = unlisten()
      if (result && typeof result.catch === 'function') {
        result.catch((error) => logError('unlisten failed', error))
      }
    } catch (error) {
      logError('unlisten failed', error)
    }
  }
}

async function init() {
  setIconButton($('closeHistory'), 'close', 14)
  setIconButton($('closeBookmarks'), 'close', 14)
  setIconButton($('settingsClose'), 'close', 14)
  setIconButton($('findPrev'), 'chevronUp', 14)
  setIconButton($('findNext'), 'down', 14)
  setIconButton($('findClose'), 'close', 14)
  setupSystemThemeListener()

  // Catch unhandled Promise rejections so the UI never silently stalls
  window.addEventListener('unhandledrejection', (event) => {
    logError('Unhandled promise rejection', event.reason)
    if (event.reason && !event.reason?.orbitHandled) {
      showToast('Unexpected error — Orbit recovered.')
    }
  })

  await loadPreferences()
  await loadRecentPages()
  try {
    $('aboutVersion').textContent = `Version ${await getVersion()}`
  } catch (error) {
    logError('getVersion failed', error)
  }
  bindEvents({
    $, absorb, win, closeAllPanels, closeTab, createTab, deleteBookmark,
    handleAddressKey, handleShortcut, navigate, openBookmarksPanel,
    openHistoryPanel, queueBrowserViewSync, queueHistorySearch, cleanupListeners,
    switchTab, toggleBookmark, toggleTheme, clearHistory,
    closeFindBar, findNext, findPrev,
    updateOverlay, closeAboutPanel, openSettingsPanel, closeSettingsPanel,
    setThemePreference, changeSearchEngine, changeStartupBehavior,
    saveShortcutEdits, handleNewTabSearch, retryErrorPage, errorPageHome,
    copyCurrentAddress, deleteShortcutAt,
    goHome: () => absorb(command('go_home_tab', { tabId: state.activeId }, 'Could not open home')),
    goBack: () => absorb(command('go_back', { tabId: state.activeId }, 'Could not go back')),
    goForward: () => absorb(command('go_forward', { tabId: state.activeId }, 'Could not go forward')),
    reload: () => absorb(command('reload_tab', { tabId: state.activeId }, 'Could not reload this page')),
    stop: () => absorb(command('stop_tab', { tabId: state.activeId }, 'Could not stop loading')),
  })
  await setupListeners()
  await syncBrowserView()
  updateOverlay()
  setupTabOverflowIndicators()

  const [existingTabs, activeId] = await Promise.all([
    command('get_tabs', {}, 'Could not restore open tabs'),
    command('get_active_tab', {}, 'Could not restore active tab'),
  ])

  if (state.startupBehavior === 'restore' && existingTabs.length > 0) {
    existingTabs.forEach(tab => state.tabs.set(tab.id, tab))
    state.activeId = activeId || existingTabs[0].id
    renderTabs()
    updateNav()
    const active = activeTab()
    if (active && isWebUrl(active.url)) {
      absorb(command('switch_tab', { tabId: active.id }, 'Could not restore active tab'))
    }
    return
  }

  await createTab()
}

document.addEventListener('DOMContentLoaded', () => {
  init().catch(error => {
    logError('Orbit init failed', error)
    showToast(formatError(error, 'Orbit could not start'))
  })
})
