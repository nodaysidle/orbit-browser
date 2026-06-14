import test from 'node:test'
import assert from 'node:assert/strict'

import { installDom } from './dom-shim.js'
import { el, icon } from '../../src/utils/dom.js'

test('el applies classes, text, attributes, and dataset values', () => {
  installDom()

  const node = el('button', {
    className: 'nav-btn active',
    text: 'Open',
    type: 'button',
    dataset: { tabId: 't1' },
    attrs: { 'aria-label': 'Open tab' },
  })

  assert.equal(node.tagName, 'BUTTON')
  assert.equal(node.className, 'nav-btn active')
  assert.equal(node.textContent, 'Open')
  assert.equal(node.type, 'button')
  assert.equal(node.dataset.tabId, 't1')
  assert.equal(node.getAttribute('aria-label'), 'Open tab')
})

test('icon returns a hidden SVG with expected geometry', () => {
  installDom()

  const svg = icon('settings', 20)

  assert.equal(svg.tagName, 'SVG')
  assert.equal(svg.getAttribute('viewBox'), '0 0 20 20')
  assert.equal(svg.getAttribute('aria-hidden'), 'true')
  assert.equal(svg.children.length, 2)
})
