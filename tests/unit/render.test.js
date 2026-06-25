import test from 'node:test'
import assert from 'node:assert/strict'

import { installDom } from './dom-shim.js'
import {
  renderBookmarksList,
  renderContinueTabs,
  renderHistoryList,
  renderProjectCards,
  renderRecentPages,
  renderShortcutEditor,
  renderShortcutGrid,
  renderTabs,
} from '../../src/utils/render.js'

test('renderTabs marks loading tabs as busy and updates count', () => {
  const document = installDom()
  const container = document.createElement('div')
  const tabCount = document.createElement('span')

  renderTabs({
    container,
    activeId: 't1',
    tabCount,
    tabs: [
      { id: 't1', title: 'Example', url: 'https://example.com', loading: true },
      { id: 't2', title: 'New Tab', url: '', loading: false },
    ],
  })

  assert.equal(tabCount.textContent, '2 tabs')
  assert.equal(container.children.length, 2)
  assert.equal(container.children[0].classList.contains('loading'), true)
  const activeTab = container.children[0].querySelector('.tab-main')
  const inactiveTab = container.children[1].querySelector('.tab-main')
  assert.equal(activeTab.getAttribute('role'), 'tab')
  assert.equal(activeTab.getAttribute('id'), 'tab-btn-t1')
  assert.equal(activeTab.getAttribute('aria-controls'), 'newTabPage')
  assert.equal(activeTab.getAttribute('aria-selected'), 'true')
  assert.equal(activeTab.getAttribute('tabindex'), '0')
  assert.equal(inactiveTab.getAttribute('aria-selected'), 'false')
  assert.equal(inactiveTab.getAttribute('tabindex'), '-1')
  assert.equal(activeTab.getAttribute('aria-busy'), 'true')
})

test('panel renderers show useful empty states', () => {
  const document = installDom()
  const history = document.createElement('div')
  const bookmarks = document.createElement('div')

  renderHistoryList(history, [])
  renderBookmarksList(bookmarks, [])

  assert.equal(history.querySelector('.panel-empty') !== null, true)
  assert.equal(bookmarks.querySelector('.panel-empty-illustration') !== null, true)
})

test('shortcut grid and editor render persisted shortcuts', () => {
  const document = installDom()
  const grid = document.createElement('div')
  const editor = document.createElement('div')
  const shortcuts = [{ title: 'Docs', url: 'https://docs.rs' }]

  renderShortcutGrid(grid, shortcuts)
  renderShortcutEditor(editor, shortcuts)

  // Updated for Slice 2 structure (shortcut pills wrapped for inline delete affordance)
  const firstBtn = grid.children[0].querySelector('button')
  assert.equal(firstBtn.dataset.shortcutUrl, 'https://docs.rs')
  assert.equal(firstBtn.querySelector('.shortcut-letter').textContent, 'D')
  assert.equal(editor.querySelector('[data-shortcut-title]').value, 'Docs')
  assert.equal(editor.querySelector('[data-shortcut-url-input]').value, 'https://docs.rs')
  assert.equal(editor.querySelector('[data-remove-shortcut-row]') !== null, true)
})

test('recent pages render openable cards', () => {
  const document = installDom()
  const container = document.createElement('div')

  renderRecentPages(container, [{ title: 'Rust', url: 'https://www.rust-lang.org/learn' }])

  assert.equal(container.children.length, 1)
  assert.equal(container.children[0].dataset.recentUrl, 'https://www.rust-lang.org/learn')
  assert.equal(container.children[0].querySelector('.recent-card-url').textContent, 'rust-lang.org')
})

test('project cards render resumable work', () => {
  const document = installDom()
  const container = document.createElement('div')

  renderProjectCards(container, [{
    id: 'p1',
    name: 'Orbit Browser',
    domains: ['github.com', 'localhost'],
    tabs: [{ url: 'https://github.com/nodaysidle/orbit-browser' }],
  }], 'p1')

  assert.equal(container.children.length, 1)
  const card = container.children[0].querySelector('[data-project-id]')
  assert.equal(card.dataset.projectId, 'p1')
  assert.equal(card.classList.contains('active'), true)
  assert.equal(card.querySelector('.continue-title').textContent, 'Orbit Browser')
  assert.equal(card.querySelector('.continue-url').textContent, 'github.com / localhost')
  assert.equal(card.querySelector('.continue-badge').textContent, 'Active')
  assert.equal(container.children[0].querySelector('[data-project-archive-id]').dataset.projectArchiveId, 'p1')
})

test('continue tabs render active local session cards', () => {
  const document = installDom()
  const container = document.createElement('div')

  renderContinueTabs(container, [
    { id: 'a', title: 'GitHub', url: 'https://github.com/nodaysidle' },
    { id: 'b', title: 'New Tab', url: '' },
  ], 'a')

  assert.equal(container.children.length, 1)
  assert.equal(container.children[0].dataset.continueTabId, 'a')
  assert.equal(container.children[0].classList.contains('active'), true)
  assert.equal(container.children[0].querySelector('.continue-url').textContent, 'github.com')
})
