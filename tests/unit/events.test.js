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
  'historyPanel', 'bookmarksPanel',
  'newTabSearchForm', 'shortcutsRow', 'aboutModal', 'aboutClose', 'settingsModal',
  'settingsClose', 'downloadModal', 'downloadConfirm', 'downloadCancel', 'downloadCancelIcon',
  'settingTheme', 'settingSearchEngine', 'settingStartup',
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
    absorb: promise => Promise.resolve(promise),
    closeAllPanels: () => calls.push('close-panels'),
    closeTab: id => calls.push(`close-tab:${id}`),
    createTab: () => calls.push('new-tab'),
    deleteBookmark: id => calls.push(`delete-bookmark:${id}`),
    handleAddressKey: () => calls.push('address-key'),
    handleShortcut: () => calls.push('shortcut'),
    navigate: url => calls.push(`navigate:${url}`),
    openBookmarksPanel: () => calls.push('bookmarks'),
    openHistoryPanel: () => calls.push('history'),
    queueBrowserViewSync: () => calls.push('resize'),
    queueHistorySearch: query => calls.push(`history-search:${query}`),
    cleanupListeners: () => calls.push('cleanup'),
    switchTab: id => calls.push(`switch-tab:${id}`),
    toggleBookmark: () => calls.push('bookmark'),
    toggleTheme: () => calls.push('theme'),
    clearHistory: () => calls.push('clear-history'),
    closeFindBar: () => calls.push('close-find'),
    findNext: () => calls.push('find-next'),
    findPrev: () => calls.push('find-prev'),
    updateOverlay: () => calls.push('overlay'),
    closeAboutPanel: () => calls.push('close-about'),
    openSettingsPanel: () => calls.push('settings'),
    closeSettingsPanel: () => calls.push('close-settings'),
    setThemePreference: value => calls.push(`theme:${value}`),
    changeSearchEngine: value => calls.push(`search:${value}`),
    changeStartupBehavior: value => calls.push(`startup:${value}`),
    saveShortcutEdits: () => calls.push('save-shortcuts'),
    handleNewTabSearch: event => {
      event.preventDefault()
      calls.push('new-tab-search')
    },
    retryErrorPage: () => calls.push('retry-error'),
    errorPageHome: () => calls.push('error-home'),
    deleteShortcutAt: id => calls.push(`delete-shortcut:${id}`),
    confirmDownload: () => calls.push('confirm-download'),
    cancelDownload: () => calls.push('cancel-download'),
    persistTabOrder: ids => calls.push(`persist-tabs:${ids.join(',')}`),
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

test('menu waits for overlay reservation before revealing dropdown items', async () => {
  const { nodes } = setup()
  const item = nodes.menuSettings
  item.classList.add('dropdown-item')
  nodes.menuDropdown.append(item)
  const visibilityAtOverlay = []
  const bound = actions({
    updateOverlay: async () => {
      visibilityAtOverlay.push(nodes.menuDropdown.style.visibility)
      await Promise.resolve()
      bound.calls.push('overlay')
    },
  })

  bindEvents(bound)
  nodes.btnMenu.dispatchEvent({ type: 'click', stopPropagation: () => bound.calls.push('stop') })

  assert.equal(nodes.menuDropdown.classList.contains('hidden'), false)
  assert.equal(nodes.menuDropdown.style.visibility, 'hidden')
  assert.deepEqual(visibilityAtOverlay, ['hidden'])

  await Promise.resolve()
  await Promise.resolve()

  assert.equal(nodes.menuDropdown.style.visibility, '')
  assert.equal(nodes.btnMenu.getAttribute('aria-expanded'), 'true')
  assert.equal(globalThis.document.activeElement, item)
  assert.equal(bound.calls.includes('overlay'), true)
})

test('bindEvents delegates dynamic shortcut and recent page clicks', () => {
  const { document, nodes } = setup()
  const bound = actions()
  const shortcut = document.createElement('button')
  shortcut.dataset.shortcutUrl = 'https://example.com'
  const recent = document.createElement('button')
  recent.dataset.recentUrl = 'https://rust-lang.org'

  bindEvents(bound)
  nodes.shortcutsRow.dispatchEvent({ type: 'click', target: shortcut })
  nodes.recentPages.dispatchEvent({ type: 'click', target: recent })

  assert.deepEqual(bound.calls.slice(-2), ['navigate:https://example.com', 'navigate:https://rust-lang.org'])
})

test('bindEvents deletes shortcuts without wrapping the action in a thunk', () => {
  const { document, nodes } = setup()
  const bound = actions()
  const deleteButton = document.createElement('button')
  deleteButton.dataset.deleteShortcut = '0'

  bindEvents(bound)
  nodes.shortcutsRow.dispatchEvent({ type: 'click', target: deleteButton })

  assert.equal(bound.calls.at(-1), 'delete-shortcut:0')
})

test('bindEvents persists tab drag order through actions', () => {
  const { document, nodes } = setup()
  const bound = actions()
  const tabA = document.createElement('button')
  tabA.dataset.tabId = 'a'
  const tabB = document.createElement('button')
  tabB.dataset.tabId = 'b'
  nodes.tabsContainer.append(tabB, tabA)

  bindEvents(bound)
  nodes.tabsContainer.dispatchEvent({ type: 'dragend' })

  assert.equal(bound.calls.at(-1), 'persist-tabs:b,a')
})

test('bindEvents arrow keys move tab focus and switch active tab', () => {
  const { document, nodes } = setup()
  const bound = actions()
  const tabA = document.createElement('button')
  tabA.dataset.tabId = 'a'
  const tabB = document.createElement('button')
  tabB.dataset.tabId = 'b'
  nodes.tabsContainer.append(tabA, tabB)

  bindEvents(bound)
  tabA.focus()
  nodes.tabsContainer.dispatchEvent({
    type: 'keydown',
    key: 'ArrowRight',
    preventDefault: () => bound.calls.push('prevent-default'),
  })

  assert.equal(document.activeElement, tabB)
  assert.deepEqual(bound.calls.slice(-2), ['prevent-default', 'switch-tab:b'])
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
