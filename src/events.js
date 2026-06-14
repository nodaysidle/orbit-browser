export function bindEvents(actions) {
  const { $, win } = actions
  const menuButton = $('btnMenu')
  const menu = $('menuDropdown')

  const menuItems = () => [...menu.querySelectorAll('.dropdown-item')]
  const isMenuOpen = () => !menu.classList.contains('hidden')

  function closeMenu({ returnFocus = false } = {}) {
    if (!isMenuOpen()) return
    menu.classList.add('hidden')
    menuButton.setAttribute('aria-expanded', 'false')
    actions.updateOverlay?.()
    if (returnFocus) menuButton.focus()
  }

  function openMenu({ focusItem = true, fromEnd = false } = {}) {
    if (isMenuOpen()) return
    menu.classList.remove('hidden')
    menuButton.setAttribute('aria-expanded', 'true')
    actions.updateOverlay?.()
    if (!focusItem) return
    const items = menuItems()
    const target = fromEnd ? items.at(-1) : items[0]
    target?.focus()
  }

  function moveMenuFocus(step) {
    const items = menuItems()
    if (!items.length) return
    const current = document.activeElement
    const index = items.indexOf(current)
    const start = index >= 0 ? index : 0
    const next = (start + step + items.length) % items.length
    items[next].focus()
  }

  $('btnClose')?.addEventListener('click', () => actions.absorb(win.close()))
  $('btnMinimize')?.addEventListener('click', () => actions.absorb(win.minimize()))
  $('btnMaximize')?.addEventListener('click', () => actions.absorb(win.toggleMaximize()))
  document.querySelector('.titlebar')?.addEventListener('dblclick', event => {
    if (event.target.closest('.traffic-lights')) return
    actions.absorb(win.toggleMaximize())
  })

  $('btnBack').addEventListener('click', () => actions.goBack())
  $('btnForward').addEventListener('click', () => actions.goForward())
  $('btnHome').addEventListener('click', () => actions.goHome())
  $('btnReload').addEventListener('click', () => actions.reload())
  $('btnStop').addEventListener('click', () => actions.stop())
  $('btnNewTab').addEventListener('click', () => actions.absorb(actions.createTab()))
  $('btnTheme').addEventListener('click', () => actions.absorb(actions.toggleTheme()))
  $('btnBookmark').addEventListener('click', () => actions.absorb(actions.toggleBookmark()))
  menuButton.addEventListener('click', event => {
    event.stopPropagation()
    if (isMenuOpen()) closeMenu()
    else openMenu()
  })
  menuButton.addEventListener('keydown', event => {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      openMenu({ focusItem: true, fromEnd: false })
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      openMenu({ focusItem: true, fromEnd: true })
    }
  })
  menu.addEventListener('keydown', event => {
    if (event.key === 'Escape') {
      event.preventDefault()
      closeMenu({ returnFocus: true })
      return
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      moveMenuFocus(1)
      return
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      moveMenuFocus(-1)
      return
    }
    if (event.key === 'Home') {
      event.preventDefault()
      menuItems()[0]?.focus()
      return
    }
    if (event.key === 'End') {
      event.preventDefault()
      menuItems().at(-1)?.focus()
      return
    }
    if (event.key === 'Tab') {
      closeMenu()
    }
  })
  menu.addEventListener('focusout', () => {
    window.setTimeout(() => {
      if (!menu.contains(document.activeElement) && document.activeElement !== menuButton) {
        closeMenu()
      }
    }, 0)
  })
  $('menuHistory').addEventListener('click', () => actions.absorb(actions.openHistoryPanel()))
  $('menuBookmarks').addEventListener('click', () => actions.absorb(actions.openBookmarksPanel()))
  $('menuSettings')?.addEventListener('click', actions.openSettingsPanel)
  $('menuClearHistory').addEventListener('click', () => actions.absorb(actions.clearHistory()))
  $('closeHistory').addEventListener('click', actions.closeAllPanels)
  $('closeBookmarks').addEventListener('click', actions.closeAllPanels)

  $('tabsContainer').addEventListener('click', event => {
    const close = event.target.closest('[data-close-tab]')
    if (close) {
      event.stopPropagation()
      actions.absorb(actions.closeTab(close.dataset.closeTab))
      return
    }
    const tab = event.target.closest('[data-tab-id]')
    if (tab) actions.absorb(actions.switchTab(tab.dataset.tabId))
  })

  // Slice 7 (frontend foundation only) — vanilla tab drag reorder
  // Full persistence requires explicit Rust command + user "ask" per approved design + AGENTS.md
  let dragSrc = null
  $('tabsContainer').addEventListener('dragstart', e => {
    const tabEl = e.target.closest('.tab')
    if (tabEl) {
      dragSrc = tabEl
      e.dataTransfer.effectAllowed = 'move'
      tabEl.style.opacity = '0.5'
    }
  })
  $('tabsContainer').addEventListener('dragover', e => {
    e.preventDefault()
    const tabEl = e.target.closest('.tab')
    if (tabEl && tabEl !== dragSrc) {
      const rect = tabEl.getBoundingClientRect()
      const mid = rect.left + rect.width / 2
      if (e.clientX < mid) {
        tabEl.parentNode.insertBefore(dragSrc, tabEl)
      } else {
        tabEl.parentNode.insertBefore(dragSrc, tabEl.nextSibling)
      }
    }
  })
  $('tabsContainer').addEventListener('dragend', () => {
    if (dragSrc) dragSrc.style.opacity = ''
    dragSrc = null
    // Visual reorder complete. Persistence layer intentionally left for explicit ask.
  })

  $('historyList').addEventListener('click', event => {
    const row = event.target.closest('[data-history-url]')
    if (row) actions.absorb(actions.navigate(row.dataset.historyUrl))
  })

  $('recentPages')?.addEventListener('click', event => {
    const row = event.target.closest('[data-recent-url]')
    if (row) actions.absorb(actions.navigate(row.dataset.recentUrl))
  })

  $('bookmarksList').addEventListener('click', event => {
    const remove = event.target.closest('[data-bookmark-delete]')
    if (remove) {
      actions.absorb(actions.deleteBookmark(remove.dataset.bookmarkDelete))
      return
    }
    const row = event.target.closest('[data-bookmark-open]')
    if (row) actions.absorb(actions.navigate(row.dataset.bookmarkOpen))
  })

  $('historySearch').addEventListener('input', event => actions.queueHistorySearch(event.target.value.trim()))
  $('addressInput').addEventListener('keydown', actions.handleAddressKey)
  $('addressInput').addEventListener('focus', event => event.target.select())

  // Slice 1 address copy (button + click-to-copy on preview tooltip area)
  $('btnCopyAddress')?.addEventListener('click', () => actions.copyCurrentAddress?.())
  const addressWrap = $('addressWrap')
  if (addressWrap) {
    addressWrap.addEventListener('click', event => {
      const isPreviewClick = event.target === addressWrap && addressWrap.getAttribute('data-url-preview')
      if (isPreviewClick) {
        actions.copyCurrentAddress?.()
      }
    })
  }
  $('newTabSearchForm')?.addEventListener('submit', actions.handleNewTabSearch)
  $('shortcutsRow')?.addEventListener('click', event => {
    const del = event.target.closest('[data-delete-shortcut]')
    if (del) {
      const idx = parseInt(del.dataset.deleteShortcut, 10)
      if (!Number.isNaN(idx)) actions.absorb(Promise.resolve().then(() => actions.deleteShortcutAt(idx)))
      return
    }
    const button = event.target.closest('[data-shortcut-url]')
    if (button) actions.absorb(actions.navigate(button.dataset.shortcutUrl))
  })

  // Slice 2 — keyboard arrow navigation on shortcuts (approved design)
  $('shortcutsRow')?.addEventListener('keydown', event => {
    if (!['ArrowLeft', 'ArrowRight'].includes(event.key)) return
    const pills = [...$('shortcutsRow').querySelectorAll('.shortcut-btn')]
    const current = document.activeElement
    const idx = pills.indexOf(current)
    if (idx === -1) return
    event.preventDefault()
    const nextIdx = event.key === 'ArrowRight' ? (idx + 1) % pills.length : (idx - 1 + pills.length) % pills.length
    pills[nextIdx].focus()
  })
  document.addEventListener('keydown', event => {
    if (event.key === 'Escape' && !$('aboutModal')?.classList.contains('hidden')) {
      actions.closeAboutPanel()
    }
    if (event.key === 'Escape' && !$('settingsModal')?.classList.contains('hidden')) {
      actions.closeSettingsPanel()
    }
  })
  document.addEventListener('keydown', actions.handleShortcut)
  $('aboutClose')?.addEventListener('click', actions.closeAboutPanel)
  $('aboutModal')?.addEventListener('click', event => {
    if (event.target === event.currentTarget) actions.closeAboutPanel()
  })
  $('settingsClose')?.addEventListener('click', actions.closeSettingsPanel)
  $('settingsModal')?.addEventListener('click', event => {
    if (event.target === event.currentTarget) actions.closeSettingsPanel()
  })
  $('settingTheme')?.addEventListener('change', event => actions.absorb(actions.setThemePreference(event.target.value)))
  $('settingSearchEngine')?.addEventListener('change', event => actions.absorb(actions.changeSearchEngine(event.target.value)))
  $('settingStartup')?.addEventListener('change', event => actions.absorb(actions.changeStartupBehavior(event.target.value)))
  $('saveShortcuts')?.addEventListener('click', () => actions.absorb(actions.saveShortcutEdits()))
  $('errorRetry')?.addEventListener('click', actions.retryErrorPage)
  $('errorHome')?.addEventListener('click', actions.errorPageHome)
  document.addEventListener('click', event => {
    if (!event.target.closest('.dropdown') && !event.target.closest('#btnMenu')) actions.closeAllPanels()
  })
  window.addEventListener('resize', actions.queueBrowserViewSync)
  window.addEventListener('beforeunload', actions.cleanupListeners)

  // Find bar
  let findDebounceTimer = null
  $('findInput').addEventListener('input', () => {
    clearTimeout(findDebounceTimer)
    findDebounceTimer = window.setTimeout(() => actions.absorb(actions.findNext()), 120)
  })
  $('findInput').addEventListener('keydown', event => {
    if (event.key === 'Enter') {
      clearTimeout(findDebounceTimer)
      actions.absorb(actions.findNext())
    }
    if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      actions.closeFindBar()
    }
  })
  $('findNext').addEventListener('click', () => actions.absorb(actions.findNext()))
  $('findPrev').addEventListener('click', () => actions.absorb(actions.findPrev()))
  $('findClose').addEventListener('click', actions.closeFindBar)
}
