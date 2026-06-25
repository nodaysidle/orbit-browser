import test from 'node:test'
import assert from 'node:assert/strict'

import { installDom } from './dom-shim.js'
import { createFindController } from '../../src/find.js'

function setupFindDom() {
  const document = installDom()
  const ids = ['findBar', 'findInput', 'findStatus']
  for (const id of ids) {
    const node = document.createElement('input')
    node.id = id
    document.body.append(node)
  }
  document.getElementById('findBar').classList.add('hidden')
  return document
}

test('findNext sends the current query to the backend command', async () => {
  const document = setupFindDom()
  const calls = []
  const controller = createFindController({
    $: id => document.getElementById(id),
    state: {
      activeId: 't1',
      tabs: new Map([['t1', { id: 't1', url: 'https://example.com' }]]),
    },
    command: (name, payload) => {
      calls.push({ name, payload })
      return Promise.resolve()
    },
    absorb: () => {},
    updateOverlay: () => {},
  })

  document.getElementById('findInput').value = 'orbit'
  await controller.findNext()

  assert.deepEqual(calls, [{
    name: 'find_in_page',
    payload: { tabId: 't1', query: 'orbit', backwards: false },
  }])
})

test('closeFindBar skips backend calls when the active tab has no web page', async () => {
  const document = setupFindDom()
  const calls = []
  const controller = createFindController({
    $: id => document.getElementById(id),
    state: {
      activeId: 't1',
      tabs: new Map([['t1', { id: 't1', url: '' }]]),
    },
    command: (name, payload) => {
      calls.push({ name, payload })
      return Promise.resolve()
    },
    absorb: () => {},
    updateOverlay: () => {},
  })

  await controller.closeFindBar()

  assert.equal(document.getElementById('findBar').classList.contains('hidden'), true)
  assert.deepEqual(calls, [])
})
