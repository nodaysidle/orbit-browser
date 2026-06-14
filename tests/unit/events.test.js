import test from 'node:test'
import assert from 'node:assert/strict'

import { installDom } from './dom-shim.js'
import { bindEvents } from '../../src/events.js'

const IDS = [
  'btnMenu', 'menuDropdown', 'btnClose', 'btnMinimize', 'btnMaximize',
  'btnBack', 'btnForward', 'btnHome', 'btnReload', 'btnStop', 'btnNewTab',
  'btnTheme', 'btnBookmark', 'menuHistory', 'menuBookmarks', 'menuSettings',
  'menuClearHistory', 'closeHistory', 'closeBookmarks', 'tabsContainer',
  'historyList', 'bookmarksList', 'recentPages', 'historySearch', 'addressInput',
  'newTabSearchForm', 'shortcutsRow', 'aboutModal', 'aboutClose', 'settingsModal',
  'settingsClose', 'settingTheme', 'settingSearchEngine', 'settingStartup',
  'saveShortcuts', 'errorRetry', 'errorHome', 'findInput', 'findNext',
  'findPrev', 'findClose', 'findBar',
]

function setup() {
  const document = installDom()
  const nodes = {}
  for (const id of IDS) {
    nodes[id] = document.createElement(id.startsWith('setting') ? 'select' : 'button')
    nodes[id].id = id
    nodes[id].setAttribute('id', id)
    document.body.append(nodes[id])
  }
  nodes.menuDropdown.classList.add('hidden')
  nodes.aboutModal.classList.add('hidden')
  nodes.settingsModal.classList.add('hidden')
  nodes.findBar.classList.add('hidden')
  const titlebar = document.createElement('div')
  titlebar.className = 'titlebar'
  document.body.append(titlebar)
  return { document, nodes }
}

function actions(overrides = {}) {
  const calls = []
  return {
    calls,
    $: id => globalThis.document.getElementById(id),
    win: { close: async () => calls.push('close'), minimize: async () => calls.push('minimize'), toggleMaximize: async () => calls.push('zoom') },
    absorb: promise => {
      assert.equal(typeof promise?.catch, 'function')
      return promise.catch(error => calls.push(`absorbed:${error?.message || error}`))
    },
    closeAllPanels: () => calls.push('close-panels'),
    closeTab: id => Promise.resolve(calls.push(`close-tab:${id}`)),
    createTab: () => Promise.resolve(calls.push('new-tab')),
    deleteBookmark: id => Promise.resolve(calls.push(`delete-bookmark:${id}`)),
    handleAddressKey: () => calls.push('address-key'),
    handleShortcut: () => calls.push('shortcut'),
    navigate: url => Promise.resolve(calls.push(`navigate:${url}`)),
    openBookmarksPanel: () => Promise.resolve(calls.push('bookmarks')),
    openHistoryPanel: () => Promise.resolve(calls.push('history')),
    queueBrowserViewSync: () => calls.push('resize'),
    queueHistorySearch: query => calls.push(`history-search:${query}`),
    cleanupListeners: () => calls.push('cleanup'),
    switchTab: id => Promise.resolve(calls.push(`switch-tab:${id}`)),
    toggleBookmark: () => Promise.resolve(calls.push('bookmark')),
    toggleTheme: () => Promise.resolve(calls.push('theme')),
    clearHistory: () => Promise.resolve(calls.push('clear-history')),
    closeFindBar: () => calls.push('close-find'),
    findNext: () => Promise.resolve(calls.push('find-next')),
    findPrev: () => Promise.resolve(calls.push('find-prev')),
    updateOverlay: () => calls.push('overlay'),
    closeAboutPanel: () => calls.push('close-about'),
    openSettingsPanel: () => calls.push('settings'),
    closeSettingsPanel: () => calls.push('close-settings'),
    setThemePreference: value => Promise.resolve(calls.push(`theme:${value}`)),
    changeSearchEngine: value => Promise.resolve(calls.push(`search:${value}`)),
    changeStartupBehavior: value => Promise.resolve(calls.push(`startup:${value}`)),
    saveShortcutEdits: () => Promise.resolve(calls.push('save-shortcuts')),
    handleNewTabSearch: event => {
      event.preventDefault()
      calls.push('new-tab-search')
    },
    retryErrorPage: () => calls.push('retry-error'),
    errorPageHome: () => calls.push('error-home'),
    deleteShortcutAt: index => Promise.resolve(calls.push(`delete-shortcut:${index}`)),
    goHome: () => calls.push('home'),
    goBack: () => calls.push('back'),
    goForward: () => calls.push('forward'),
    reload: () => calls.push('reload'),
    stop: () => calls.push('stop'),
    ...overrides,
  }
}

test('bindEvents wires primary browser controls', () => {
  const { nodes } = setup()
  const bound = actions()

  bindEvents(bound)
  nodes.btnNewTab.dispatchEvent({ type: 'click' })
  nodes.menuSettings.dispatchEvent({ type: 'click' })
  nodes.errorRetry.dispatchEvent({ type: 'click' })

  assert.deepEqual(bound.calls.slice(-3), ['new-tab', 'settings', 'retry-error'])
})

test('bindEvents delegates dynamic shortcut, shortcut delete, and recent page clicks', async () => {
  const { document, nodes } = setup()
  const bound = actions()
  const shortcut = document.createElement('button')
  shortcut.dataset.shortcutUrl = 'https://example.com'
  const shortcutDelete = document.createElement('button')
  shortcutDelete.dataset.deleteShortcut = '0'
  const recent = document.createElement('button')
  recent.dataset.recentUrl = 'https://rust-lang.org'

  bindEvents(bound)
  nodes.shortcutsRow.dispatchEvent({ type: 'click', target: shortcut })
  nodes.shortcutsRow.dispatchEvent({ type: 'click', target: shortcutDelete })
  nodes.recentPages.dispatchEvent({ type: 'click', target: recent })
  await Promise.resolve()

  assert.deepEqual(bound.calls.slice(-3), ['navigate:https://example.com', 'navigate:https://rust-lang.org', 'delete-shortcut:0'])
})

test('bindEvents routes synchronous shortcut delete failures through absorb', async () => {
  const { document, nodes } = setup()
  const bound = actions({
    deleteShortcutAt: () => {
      throw new Error('delete exploded')
    },
  })
  const shortcutDelete = document.createElement('button')
  shortcutDelete.dataset.deleteShortcut = '0'

  bindEvents(bound)
  assert.doesNotThrow(() => nodes.shortcutsRow.dispatchEvent({ type: 'click', target: shortcutDelete }))
  await Promise.resolve()
  await Promise.resolve()

  assert.equal(bound.calls.at(-1), 'absorbed:delete exploded')
})

test('bindEvents persists settings changes through actions', () => {
  const { nodes } = setup()
  const bound = actions()

  bindEvents(bound)
  nodes.settingTheme.value = 'light'
  nodes.settingTheme.dispatchEvent({ type: 'change', target: nodes.settingTheme })
  nodes.settingSearchEngine.value = 'brave'
  nodes.settingSearchEngine.dispatchEvent({ type: 'change', target: nodes.settingSearchEngine })
  nodes.saveShortcuts.dispatchEvent({ type: 'click' })

  assert.deepEqual(bound.calls.slice(-3), ['theme:light', 'search:brave', 'save-shortcuts'])
})
