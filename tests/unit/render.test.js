import test from 'node:test'
import assert from 'node:assert/strict'

import { installDom } from './dom-shim.js'
import {
  renderBookmarksList,
  renderHistoryList,
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
  assert.equal(container.children[0].querySelector('.tab-main').getAttribute('aria-busy'), 'true')
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
})

test('recent pages render openable cards', () => {
  const document = installDom()
  const container = document.createElement('div')

  renderRecentPages(container, [{ title: 'Rust', url: 'https://www.rust-lang.org/learn' }])

  assert.equal(container.children.length, 1)
  assert.equal(container.children[0].dataset.recentUrl, 'https://www.rust-lang.org/learn')
  assert.equal(container.children[0].querySelector('.recent-card-url').textContent, 'rust-lang.org')
})
