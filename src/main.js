import { invoke } from '@tauri-apps/api/core'
import { listen }  from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

// ── State ─────────────────────────────────────────────────────────────────────
const state = {
  tabs: new Map(),   // id -> TabInfo
  activeId: null,
}

// ── DOM helpers ───────────────────────────────────────────────────────────────
const $ = id => document.getElementById(id)
const win = getCurrentWindow()

// ── Rendering ─────────────────────────────────────────────────────────────────
function renderTabs() {
  const container = $('tabsContainer')
  const activeId  = state.activeId
  const tabs      = [...state.tabs.values()]

  container.innerHTML = tabs.map(tab => `
    <div class="tab${tab.id === activeId ? ' active' : ''}${tab.loading ? ' loading' : ''}"
         data-id="${tab.id}">
      <div class="tab-favicon">${faviconHtml(tab)}</div>
      <span class="tab-title">${escHtml(tab.title || 'New Tab')}</span>
      <button class="tab-close" data-close="${tab.id}" aria-label="Close tab">
        <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
          <line x1="1" y1="1" x2="7" y2="7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
          <line x1="7" y1="1" x2="1" y2="7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
  `).join('')

  // Tab count
  $('tabCount').textContent = `${tabs.length} tab${tabs.length !== 1 ? 's' : ''}`

  // Wire click events
  container.querySelectorAll('.tab').forEach(el => {
    el.addEventListener('click', e => {
      if (!e.target.closest('.tab-close')) switchTab(el.dataset.id)
    })
  })
  container.querySelectorAll('.tab-close').forEach(btn => {
    btn.addEventListener('click', e => { e.stopPropagation(); closeTab(btn.dataset.close) })
  })
}

function faviconHtml(tab) {
  if (!tab.url || !tab.url.startsWith('http')) {
    return '<div class="tab-monogram"> </div>'
  }
  const letter = (tab.title || tab.url).trim()[0]?.toUpperCase() || '?'
  const domain = (() => { try { return new URL(tab.url).hostname } catch { return '' } })()
  return `
    <img src="https://www.google.com/s2/favicons?domain=${domain}&sz=32"
         width="14" height="14"
         onerror="this.style.display='none';this.nextElementSibling.style.display='flex'"
         style="border-radius:2px" />
    <div class="tab-monogram" style="display:none">${letter}</div>
  `
}

function updateNav() {
  const tab = state.tabs.get(state.activeId)
  const url = tab?.url || ''
  $('addressInput').value  = url.startsWith('http') ? url : ''
  $('btnBack').disabled    = !tab?.can_go_back
  $('btnForward').disabled = !tab?.can_go_forward
  const isHttps = url.startsWith('https://')
  $('lockIcon').style.opacity = isHttps ? '1' : '0.3'
  updateBookmarkIcon()
  // Show/hide new tab page
  const showPage = !tab || !tab.url || !tab.url.startsWith('http')
  $('newTabPage').classList.toggle('hidden', !showPage)
}

async function updateBookmarkIcon() {
  const tab = state.tabs.get(state.activeId)
  if (!tab?.url?.startsWith('http')) return
  try {
    const marked = await invoke('is_bookmarked', { url: tab.url })
    $('btnBookmark').classList.toggle('bookmarked', marked)
  } catch {}
}

function escHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;')
}

// ── Actions ───────────────────────────────────────────────────────────────────
async function createTab(url = '') {
  try {
    const tab = await invoke('create_tab', { url, makeActive: true })
    state.tabs.set(tab.id, tab)
    state.activeId = tab.id
    renderTabs()
    updateNav()
    if (!url) $('addressInput').focus()
    return tab
  } catch (e) { console.error('create_tab failed', e) }
}

async function switchTab(tabId) {
  if (tabId === state.activeId) return
  try {
    await invoke('switch_tab', { tabId })
    state.activeId = tabId
    renderTabs()
    updateNav()
  } catch (e) { console.error('switch_tab failed', e) }
}

async function closeTab(tabId) {
  try {
    const newActiveId = await invoke('close_tab', { tabId })
    state.tabs.delete(tabId)
    if (state.tabs.size === 0) { await createTab(); return }
    if (state.activeId === tabId && newActiveId) {
      state.activeId = newActiveId
      if (newActiveId) await invoke('switch_tab', { tabId: newActiveId })
    }
    renderTabs()
    updateNav()
  } catch (e) { console.error('close_tab failed', e) }
}

async function navigate(rawUrl) {
  if (!rawUrl?.trim()) return
  try {
    await invoke('navigate_tab', { tabId: state.activeId, url: rawUrl })
  } catch (e) { console.error('navigate_tab failed', e) }
}

// ── Bookmark / History panels ─────────────────────────────────────────────────
let historySearchTimer = null

async function openHistoryPanel() {
  closeAllPanels()
  const entries = await invoke('get_history', { limit: 50, offset: 0 })
  renderHistoryList(entries)
  $('historyPanel').classList.remove('hidden')
}

function renderHistoryList(entries) {
  $('historyList').innerHTML = entries.map(e => `
    <div class="panel-row" data-url="${escHtml(e.url)}">
      <div class="panel-row-favicon">${(e.url[8]||'?').toUpperCase()}</div>
      <div class="panel-row-text">
        <div class="panel-row-title">${escHtml(e.title)}</div>
        <div class="panel-row-url">${escHtml(e.url)}</div>
      </div>
    </div>
  `).join('')
  $('historyList').querySelectorAll('.panel-row').forEach(row => {
    row.addEventListener('click', () => { navigate(row.dataset.url); closeAllPanels() })
  })
}

async function openBookmarksPanel() {
  closeAllPanels()
  const bookmarks = await invoke('get_bookmarks')
  $('bookmarksList').innerHTML = bookmarks.map(b => `
    <div class="panel-row" data-url="${escHtml(b.url)}" data-id="${b.id}">
      <div class="panel-row-favicon">${(b.title[0]||'?').toUpperCase()}</div>
      <div class="panel-row-text">
        <div class="panel-row-title">${escHtml(b.title)}</div>
        <div class="panel-row-url">${escHtml(b.url)}</div>
      </div>
      <button class="panel-row-delete" data-bmid="${b.id}">✕</button>
    </div>
  `).join('')
  $('bookmarksList').querySelectorAll('.panel-row').forEach(row => {
    row.addEventListener('click', e => {
      if (e.target.closest('.panel-row-delete')) return
      navigate(row.dataset.url); closeAllPanels()
    })
  })
  $('bookmarksList').querySelectorAll('.panel-row-delete').forEach(btn => {
    btn.addEventListener('click', async e => {
      e.stopPropagation()
      await invoke('delete_bookmark', { id: btn.dataset.bmid })
      btn.closest('.panel-row').remove()
    })
  })
  $('bookmarksPanel').classList.remove('hidden')
}

function closeAllPanels() {
  ['menuDropdown','historyPanel','bookmarksPanel'].forEach(id => $(id).classList.add('hidden'))
}

// ── Tauri events ──────────────────────────────────────────────────────────────
async function setupListeners() {
  await listen('tab-navigating', ({ payload }) => {
    const tab = state.tabs.get(payload.id)
    if (tab && payload.id === state.activeId) {
      $('addressInput').value = payload.url
    }
  })

  await listen('tab-loading', ({ payload }) => {
    const tab = state.tabs.get(payload.id)
    if (tab) {
      tab.loading = true
      tab.url = payload.url
      if (payload.id === state.activeId) {
        $('addressInput').value = payload.url
        $('newTabPage').classList.add('hidden')
      }
      renderTabs()
    }
  })

  await listen('tab-loaded', ({ payload }) => {
    state.tabs.set(payload.id, { ...state.tabs.get(payload.id), ...payload, loading: false })
    renderTabs()
    if (payload.id === state.activeId) updateNav()
  })
}

// ── Init ──────────────────────────────────────────────────────────────────────
async function init() {
  // Window controls
  $('btnClose')   ?.addEventListener('click', () => win.close())
  $('btnMinimize')?.addEventListener('click', () => win.minimize())
  $('btnMaximize')?.addEventListener('click', () => win.toggleMaximize())

  // Nav buttons
  $('btnBack')   .addEventListener('click', () => invoke('go_back',     { tabId: state.activeId }))
  $('btnForward').addEventListener('click', () => invoke('go_forward',  { tabId: state.activeId }))
  $('btnReload') .addEventListener('click', () => invoke('reload_tab',  { tabId: state.activeId }))
  $('btnNewTab') .addEventListener('click', () => createTab())

  // Bookmark
  $('btnBookmark').addEventListener('click', async () => {
    const tab = state.tabs.get(state.activeId)
    if (!tab?.url?.startsWith('http')) return
    const marked = await invoke('is_bookmarked', { url: tab.url })
    if (marked) {
      const bookmarks = await invoke('get_bookmarks')
      const bm = bookmarks.find(b => b.url === tab.url)
      if (bm) { await invoke('delete_bookmark', { id: bm.id }); $('btnBookmark').classList.remove('bookmarked') }
    } else {
      await invoke('add_bookmark', { url: tab.url, title: tab.title || tab.url })
      $('btnBookmark').classList.add('bookmarked')
    }
  })

  // Menu
  $('btnMenu').addEventListener('click', e => {
    e.stopPropagation()
    $('menuDropdown').classList.toggle('hidden')
  })
  $('menuHistory')  .addEventListener('click', () => openHistoryPanel())
  $('menuBookmarks').addEventListener('click', () => openBookmarksPanel())
  $('menuClearHistory').addEventListener('click', async () => {
    await invoke('clear_history'); closeAllPanels()
  })
  $('closeHistory')  .addEventListener('click', closeAllPanels)
  $('closeBookmarks').addEventListener('click', closeAllPanels)

  // History search (debounced 150ms)
  $('historySearch').addEventListener('input', e => {
    clearTimeout(historySearchTimer)
    historySearchTimer = setTimeout(async () => {
      const q = e.target.value.trim()
      const entries = q.length > 1
        ? await invoke('search_history', { query: q })
        : await invoke('get_history', { limit: 50, offset: 0 })
      renderHistoryList(entries)
    }, 150)
  })

  // Address bar
  $('addressInput').addEventListener('keydown', e => {
    if (e.key === 'Enter') { navigate(e.target.value); e.target.blur() }
    if (e.key === 'Escape') { updateNav(); e.target.blur() }
  })
  $('addressInput').addEventListener('focus', e => e.target.select())

  // Shortcuts on new-tab page
  document.querySelectorAll('.shortcut-btn').forEach(btn => {
    btn.addEventListener('click', () => navigate(btn.dataset.url))
  })

  // Keyboard shortcuts
  document.addEventListener('keydown', e => {
    const mod = e.metaKey || e.ctrlKey
    if (!mod) return
    switch (e.key) {
      case 't': e.preventDefault(); createTab(); break
      case 'w': e.preventDefault(); closeTab(state.activeId); break
      case 'l': e.preventDefault(); $('addressInput').focus(); break
      case 'r': e.preventDefault(); invoke('reload_tab', { tabId: state.activeId }); break
      case '[': e.preventDefault(); invoke('go_back',    { tabId: state.activeId }); break
      case ']': e.preventDefault(); invoke('go_forward', { tabId: state.activeId }); break
      default:
        if (e.key >= '1' && e.key <= '9') {
          e.preventDefault()
          const tabs = [...state.tabs.values()]
          const idx = parseInt(e.key) - 1
          if (tabs[idx]) switchTab(tabs[idx].id)
        }
    }
  })

  // Close dropdowns on outside click
  document.addEventListener('click', e => {
    if (!e.target.closest('.dropdown') && !e.target.closest('#btnMenu')) {
      closeAllPanels()
    }
  })

  // Setup Rust event listeners
  await setupListeners()

  // Load existing state
  const [existingTabs, activeId] = await Promise.all([
    invoke('get_tabs'),
    invoke('get_active_tab'),
  ])

  if (existingTabs.length > 0) {
    existingTabs.forEach(tab => state.tabs.set(tab.id, tab))
    state.activeId = activeId || existingTabs[0].id
  } else {
    await createTab()
    return
  }

  renderTabs()
  updateNav()
}

document.addEventListener('DOMContentLoaded', init)
