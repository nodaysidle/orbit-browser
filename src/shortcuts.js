import { isEditableTarget } from './utils/ui.js'

const EDITABLE_SHORTCUT_KEYS = new Set(['l', 'r', 'f', 'g', '[', ']', '.', 'h'])

const SHORTCUT_ACTIONS = new Map([
  ['t', { type: 'new-tab' }],
  [',', { type: 'settings' }],
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
  ['.', { type: 'stop' }],
  ['[', { type: 'back' }],
  [']', { type: 'forward' }],
  ['shift+[', { type: 'previous-tab' }],
  ['shift+]', { type: 'next-tab' }],
  ['y', { type: 'show-history' }],
])

const OPTION_SHORTCUT_ACTIONS = new Map([
  ['b', { type: 'show-bookmarks' }],
])

const TAB_MOVE_ACTIONS = new Map([
  ['arrowleft', { type: 'move-active-tab', direction: -1 }],
  ['arrowright', { type: 'move-active-tab', direction: 1 }],
])

export function getShortcutIntent(event) {
  if (event.key === 'Escape') return { type: 'escape' }
  if (!event.metaKey && event.ctrlKey && event.key === 'Tab') {
    return { type: event.shiftKey ? 'previous-tab' : 'next-tab' }
  }
  const mod = event.metaKey || event.ctrlKey
  if (!mod) return null

  const key = event.key.toLowerCase()
  if (event.altKey && event.shiftKey && event.metaKey) return TAB_MOVE_ACTIONS.get(key) || null
  if (event.altKey && event.metaKey && !event.shiftKey) return OPTION_SHORTCUT_ACTIONS.get(key) || null
  if (isEditableTarget(event.target) && !EDITABLE_SHORTCUT_KEYS.has(key)) return null
  if (event.key >= '1' && event.key <= '9') {
    return { type: 'switch-tab-index', index: Number.parseInt(event.key, 10) - 1 }
  }
  return SHORTCUT_ACTIONS.get(`${event.shiftKey ? 'shift+' : ''}${key}`) || null
}

export function createShortcutHandlers({
  $, absorb, command, state, createTab, closeTab, openFindBar, findNext,
  openBookmarksPanel, openHistoryPanel, openSettingsPanel,
  cycleTab, moveActiveTab, switchTab, zoomIn, zoomOut, resetZoom, goHome,
}) {
  return {
    'new-tab': () => absorb(createTab()),
    home: () => absorb(goHome()),
    'close-tab': () => absorb(closeTab(state.activeId)),
    'focus-address': () => $('addressInput').focus(),
    reload: () => absorb(command('reload_tab', { tabId: state.activeId }, 'Could not reload this page')),
    find: () => openFindBar(),
    'find-next': () => findNext(),
    'zoom-in': () => absorb(zoomIn()),
    'zoom-out': () => absorb(zoomOut()),
    'reset-zoom': () => absorb(resetZoom()),
    stop: () => absorb(command('stop_tab', { tabId: state.activeId }, 'Could not stop loading')),
    back: () => absorb(command('go_back', { tabId: state.activeId }, 'Could not go back')),
    forward: () => absorb(command('go_forward', { tabId: state.activeId }, 'Could not go forward')),
    'previous-tab': () => cycleTab(-1),
    'next-tab': () => cycleTab(1),
    'move-active-tab': intent => absorb(moveActiveTab(intent.direction)),
    'switch-tab-index': intent => {
      const tab = [...state.tabs.values()][intent.index]
      if (tab) absorb(switchTab(tab.id))
    },
    'show-bookmarks': () => absorb(openBookmarksPanel()),
    'show-history': () => absorb(openHistoryPanel()),
    settings: () => openSettingsPanel(),
  }
}

export function runShortcutIntent(intent, handlers) {
  handlers[intent.type]?.(intent)
}
