import { el, icon } from './dom.js'
import { getTabMonogram, getTabToneClass, getUrlHost } from './ui.js'

export function renderTabs({ container, tabs, activeId, tabCount }) {
  container.replaceChildren(...tabs.map(tab => tabRow(tab, activeId)))
  tabCount.textContent = `${tabs.length} tab${tabs.length === 1 ? '' : 's'}`
}

export function renderHistoryList(list, entries = []) {
  if (!entries.length) {
    list.replaceChildren(panelEmpty('clock', 'No history yet. Browse a few pages and they will show up here.'))
    return
  }

  list.replaceChildren(...entries.map(entry => panelRow({
    item: entry,
    action: 'historyUrl',
    value: entry.url,
  })))
}

export function renderBookmarksList(list, bookmarks = []) {
  if (!bookmarks.length) {
    list.replaceChildren(panelEmpty('star', 'No bookmarks yet. Use the bookmark button to save the current page.'))
    return
  }

  list.replaceChildren(...bookmarks.map(bookmark => {
    const row = el('div', { className: 'panel-row shell' })
    const main = panelRow({
      item: bookmark,
      action: 'bookmarkOpen',
      value: bookmark.url,
      className: 'panel-row panel-row-main',
    })
    const remove = el('button', {
      className: 'panel-row-delete',
      type: 'button',
      dataset: { bookmarkDelete: bookmark.id },
      attrs: { 'aria-label': `Delete ${bookmark.title || bookmark.url}` },
    }, [icon('trash', 16)])
    row.replaceChildren(main, remove)
    return row
  }))
}

export function renderShortcutGrid(container, shortcuts = []) {
  const tones = ['tone-blue', 'tone-violet', 'tone-gold', 'tone-teal']
  container.replaceChildren(...shortcuts.map((shortcut, index) => {
    const btn = el('button', {
      className: `shortcut-btn ${tones[index % tones.length]}`,
      type: 'button',
      dataset: { url: shortcut.url, shortcutUrl: shortcut.url, shortcutIndex: String(index) },
      attrs: { 'aria-label': `Open ${shortcut.title}` },
    }, [
      el('span', { className: 'shortcut-letter', text: shortcut.title?.[0]?.toUpperCase() || 'O' }),
      el('span', { className: 'shortcut-tooltip', text: shortcut.title }),
    ])

    const del = el('button', {
      className: 'shortcut-delete',
      type: 'button',
      dataset: { deleteShortcut: String(index) },
      attrs: { 'aria-label': `Delete ${shortcut.title}` },
    }, [icon('close', 10)])

    const wrap = el('div', { className: 'shortcut-wrap' }, [btn, del])
    return wrap
  }))
}

export function renderRecentPages(container, entries = []) {
  if (!entries.length) {
    container.replaceChildren(
      el('div', { className: 'recent-empty recent-empty-with-suggestions' }, [
        icon('clock', 16),
        el('span', { text: 'No recent activity yet.' }),
      ])
    )
    return
  }

  container.replaceChildren(...entries.slice(0, 6).map(entry => {
    const title = entry.title || getUrlHost(entry.url) || entry.url
    return el('button', {
      className: 'recent-card',
      type: 'button',
      dataset: { recentUrl: entry.url },
      attrs: { 'aria-label': `Open ${title}` },
    }, [
      renderMonogram(entryWithFavicon(entry), `panel-monogram ${getTabToneClass(entry)}`),
      el('span', { className: 'recent-card-copy' }, [
        el('span', { className: 'recent-card-title', text: title }),
        el('span', { className: 'recent-card-url', text: getUrlHost(entry.url) || entry.url }),
      ]),
    ])
  }))
}

export function renderProjectCards(container, projects = [], activeProjectId = null) {
  if (!container) return
  if (!projects.length) {
    container.replaceChildren()
    return
  }
  container.replaceChildren(...projects.slice(0, 3).map(project => {
    const domains = Array.isArray(project.domains) ? project.domains.slice(0, 2).join(' / ') : ''
    const tabCount = Array.isArray(project.tabs) ? project.tabs.length : 0
    const isActive = project.id === activeProjectId
    const card = el('button', {
      className: `project-card continue-card ${isActive ? 'active' : ''}`.trim(),
      type: 'button',
      dataset: { projectId: project.id },
      attrs: { 'aria-label': `Resume ${project.name}` },
    }, [
      el('span', { className: 'project-mark panel-monogram tone-blue', text: project.name?.[0]?.toUpperCase() || 'P' }),
      el('span', { className: 'continue-meta' }, [
        el('span', { className: 'continue-title', text: project.name || 'Untitled Project' }),
        el('span', { className: 'continue-url', text: domains || `${tabCount} page${tabCount === 1 ? '' : 's'}` }),
      ]),
      el('span', { className: 'continue-badge', text: isActive ? 'Active' : 'Resume' }),
    ])
    const archive = el('button', {
      className: 'project-archive',
      type: 'button',
      text: 'Archive',
      dataset: { projectArchiveId: project.id },
      attrs: { 'aria-label': `Archive ${project.name}` },
    })
    return el('div', { className: 'project-card-wrap' }, [card, archive])
  }))
}

export function renderContinueTabs(container, tabs = [], activeId = null) {
  const webTabs = tabs.filter(tab => tab?.url).slice(0, 3)
  if (!webTabs.length) {
    container.replaceChildren(el('div', { className: 'recent-empty' }, [
      icon('clock', 16),
      el('span', { text: 'Open a page or restore a session to continue working here.' }),
    ]))
    return
  }

  container.replaceChildren(...webTabs.map(tab => {
    const title = tab.title || getUrlHost(tab.url) || tab.url
    const active = tab.id === activeId
    return el('button', {
      className: `continue-card ${active ? 'active' : ''}`.trim(),
      type: 'button',
      dataset: { continueTabId: tab.id },
      attrs: { 'aria-label': `Switch to ${title}` },
    }, [
      renderMonogram(entryWithFavicon(tab), `panel-monogram ${getTabToneClass(tab)}`),
      el('span', { className: 'continue-meta' }, [
        el('span', { className: 'continue-title', text: title }),
        el('span', { className: 'continue-url', text: getUrlHost(tab.url) || tab.url }),
      ]),
      active ? el('span', { className: 'continue-badge', text: 'Now' }) : null,
    ].filter(Boolean))
  }))
}

export function renderShortcutEditor(container, shortcuts = []) {
  container.replaceChildren(...shortcuts.map((shortcut, index) => el('div', {
    className: 'shortcut-editor-row',
    dataset: { shortcutIndex: String(index) },
  }, [
    el('input', {
      className: 'settings-input',
      attrs: {
        'aria-label': `Shortcut ${index + 1} title`,
        value: shortcut.title,
        'data-shortcut-title': String(index),
      },
    }),
    el('input', {
      className: 'settings-input',
      attrs: {
        'aria-label': `Shortcut ${index + 1} URL`,
        value: shortcut.url,
        'data-shortcut-url-input': String(index),
      },
    }),
    el('button', {
      className: 'shortcut-remove-row',
      type: 'button',
      dataset: { removeShortcutRow: String(index) },
      attrs: { 'aria-label': `Remove shortcut ${index + 1}` },
    }, [icon('close', 14)]),
  ])))
}

function tabRow(tab, activeId) {
  const tone = getTabToneClass(tab)
  const title = tab.title || 'New Tab'
  const active = tab.id === activeId ? ' active' : ''
  const loading = tab.loading ? ' loading' : ''
  const selected = tab.id === activeId
  const row = el('div', { className: `tab ${tone}${active}${loading}`, attrs: { draggable: 'true', role: 'presentation', 'aria-label': `Tab: ${title}. Drag or use Cmd+Opt+Shift+arrows to reorder.` } })
  const main = el('button', {
    className: 'tab-main',
      type: 'button',
      title: tab.url || title,
      dataset: { tabId: tab.id },
      attrs: {
        role: 'tab',
        id: `tab-btn-${tab.id}`,
        'aria-selected': selected ? 'true' : 'false',
        'aria-controls': 'newTabPage',
        'aria-label': `Switch to ${title}`,
        'aria-busy': tab.loading ? 'true' : false,
        tabindex: selected ? '0' : '-1',
      },
  }, [
    renderMonogram(tab),
    el('span', { className: 'tab-title', text: title }),
  ])
  const close = el('button', {
    className: 'tab-close',
    type: 'button',
    dataset: { closeTab: tab.id },
    attrs: { 'aria-label': `Close ${title}` },
  }, [icon('close', 14)])
  row.replaceChildren(main, close)
  return row
}

function panelRow({ item, action, value, className = 'panel-row' }) {
  const tone = getTabToneClass(item)
  const title = item.title || getUrlHost(item.url) || item.url
  const ariaLabel = item.url ? `${title}, ${item.url}` : title
  const normalized = entryWithFavicon(item)
  return el('button', {
    className,
    type: 'button',
    dataset: { [action]: value },
    attrs: { 'aria-label': ariaLabel },
  }, [
    renderMonogram(normalized, `panel-monogram ${tone}`),
    el('span', { className: 'panel-row-main' }, [
      el('span', { className: 'panel-row-title', text: title }),
      el('span', { className: 'panel-row-url', text: item.url }),
    ]),
  ])
}

function panelEmpty(iconName, message) {
  return el('div', { className: 'panel-empty' }, [
    el('span', { className: 'panel-empty-illustration' }, [icon(iconName, 22)]),
    el('span', { text: message }),
  ])
}

function entryWithFavicon(item) {
  if (!item?.favicon) return item
  return { ...item, favicon_url: item.favicon }
}

function renderMonogram(tab, className = 'tab-monogram') {
  if (tab?.favicon_url) {
    const img = el('img', {
      className: 'tab-favicon',
      attrs: {
        alt: '',
        'aria-hidden': 'true',
        loading: 'lazy',
      },
    })
    img.addEventListener('error', () => hideBrokenFavicon(img))
    img.setAttribute('src', tab.favicon_url)
    const fallback = el('span', {
      className,
      text: getTabMonogram(tab),
      attrs: { 'aria-hidden': 'true', style: 'display:none' },
    })
    const wrapper = el('span', { className: `${className} tab-icon-wrapper` }, [img, fallback])
    return wrapper
  }
  return el('span', {
    className,
    text: getTabMonogram(tab),
    attrs: { 'aria-hidden': 'true' },
  })
}

function hideBrokenFavicon(img) {
  img.style.display = 'none'
  const fallback = img.nextElementSibling
  if (fallback) fallback.style.display = ''
}
