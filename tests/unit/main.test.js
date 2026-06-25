import test from 'node:test'
import assert from 'node:assert/strict'

import { installDom } from './dom-shim.js'

async function loadMain() {
  installDom()
  return import(`../../src/main.js?test=${Date.now()}-${Math.random()}`)
}

test('getShortcutIntent resolves command shortcuts through the action map', async () => {
  const { getShortcutIntent } = await loadMain()

  assert.deepEqual(getShortcutIntent({ key: 't', metaKey: true, ctrlKey: false, shiftKey: false }), { type: 'new-tab' })
  assert.deepEqual(getShortcutIntent({ key: '[', metaKey: true, ctrlKey: false, shiftKey: true }), { type: 'previous-tab' })
  assert.deepEqual(getShortcutIntent({ key: 'ArrowRight', metaKey: true, altKey: true, ctrlKey: false, shiftKey: true }), { type: 'move-active-tab', direction: 1 })
  assert.deepEqual(getShortcutIntent({ key: '3', metaKey: true, ctrlKey: false, shiftKey: false }), { type: 'switch-tab-index', index: 2 })
  assert.deepEqual(getShortcutIntent({ key: 'Tab', metaKey: false, ctrlKey: true, shiftKey: false }), { type: 'next-tab' })
  assert.deepEqual(getShortcutIntent({ key: 'Tab', metaKey: false, ctrlKey: true, shiftKey: true }), { type: 'previous-tab' })
  assert.deepEqual(getShortcutIntent({ key: ',', metaKey: true, ctrlKey: false, shiftKey: false }), { type: 'settings' })
  assert.deepEqual(getShortcutIntent({ key: 'y', metaKey: true, ctrlKey: false, shiftKey: false }), { type: 'show-history' })
  assert.deepEqual(getShortcutIntent({ key: 'b', metaKey: true, altKey: true, ctrlKey: false, shiftKey: false }), { type: 'show-bookmarks' })
})

test('getShortcutIntent ignores destructive shortcuts inside editable fields', async () => {
  const { getShortcutIntent } = await loadMain()
  const input = { tagName: 'INPUT' }

  assert.equal(getShortcutIntent({ key: 'w', metaKey: true, ctrlKey: false, shiftKey: false, target: input }), null)
  assert.deepEqual(getShortcutIntent({ key: 'l', metaKey: true, ctrlKey: false, shiftKey: false, target: input }), { type: 'focus-address' })
})

test('applyLoadedTabToMap updates only existing tabs', async () => {
  const { applyLoadedTabToMap } = await loadMain()
  const tabs = new Map([
    ['t1', { id: 't1', title: 'Loading', url: 'https://example.com', loading: true }],
  ])

  const next = applyLoadedTabToMap(tabs, { id: 't1', title: 'Example', url: 'https://example.com', loading: true })
  const ignored = applyLoadedTabToMap(next, { id: 'missing', title: 'Late callback' })

  assert.equal(next.get('t1').loading, false)
  assert.equal(next.get('t1').title, 'Example')
  assert.equal(ignored, next)
})

test('applyProgressToMap marks in-flight navigation without creating tabs', async () => {
  const { applyProgressToMap } = await loadMain()
  const tabs = new Map([
    ['t1', { id: 't1', title: 'Old', url: 'https://old.example', loading: false }],
  ])

  const next = applyProgressToMap(tabs, { id: 't1', url: 'https://new.example' })

  assert.equal(next.get('t1').loading, true)
  assert.equal(next.get('t1').title, 'new.example')
  assert.equal(applyProgressToMap(next, { id: 'missing', url: 'https://late.example' }), next)
})

test('moveTabInMap reorders active tab without dropping tab data', async () => {
  const { moveTabInMap } = await loadMain()
  const tabs = new Map([
    ['a', { id: 'a' }],
    ['b', { id: 'b' }],
    ['c', { id: 'c' }],
  ])

  const { tabs: moved, orderedIds } = await moveTabInMap(tabs, 'b', 1)

  assert.deepEqual(orderedIds, ['a', 'c', 'b'])
  assert.deepEqual([...moved.keys()], ['a', 'c', 'b'])
  assert.equal(moved.get('b').id, 'b')
})

test('projectTabsForResume returns saved Project tabs in restore order', async () => {
  const { projectTabsForResume } = await loadMain()
  const tabs = projectTabsForResume({
    tabs: [
      { url: 'https://docs.rs', position: 2 },
      { url: 'https://github.com/nodaysidle/orbit-browser', position: 0 },
      { url: 'http://localhost:3000', position: 1, is_active: true },
    ],
  })

  assert.deepEqual(tabs.map(tab => tab.url), [
    'https://github.com/nodaysidle/orbit-browser',
    'http://localhost:3000',
    'https://docs.rs',
  ])
  assert.equal(tabs[1].is_active, true)
})
