import test from 'node:test'
import assert from 'node:assert/strict'

import { installDom } from './dom-shim.js'
import { showConfirmToast } from '../../src/utils/toast.js'

function setupToast() {
  const document = installDom()
  const timers = []
  globalThis.window.setTimeout = callback => {
    timers.push(callback)
    return timers.length
  }

  const region = document.createElement('div')
  region.id = 'toastRegion'
  region.setAttribute('id', 'toastRegion')
  document.body.append(region)

  const trigger = document.createElement('button')
  trigger.id = 'btnMenu'
  trigger.setAttribute('id', 'btnMenu')
  document.body.append(trigger)

  return {
    document,
    region,
    trigger,
    flushTimers: () => {
      while (timers.length) timers.shift()()
    },
  }
}

test('confirm toast can focus the safe action and cancel from Escape', () => {
  const { document, region, trigger, flushTimers } = setupToast()
  const calls = []
  let prevented = false

  showConfirmToast({
    message: 'Clear local browsing history from this Mac?',
    confirmLabel: 'Clear History',
    cancelLabel: 'Keep History',
    focusCancel: true,
    returnFocusTo: trigger,
    onConfirm: () => calls.push('confirm'),
    onCancel: () => calls.push('cancel'),
  })

  const toast = region.querySelector('.toast')
  const primary = region.querySelector('.toast-action-primary')
  const actions = region.querySelectorAll('.toast-action')

  assert.equal(primary.textContent, 'Clear History')
  assert.equal(actions[1].textContent, 'Keep History')
  assert.equal(document.activeElement, actions[1])

  toast.dispatchEvent({
    type: 'keydown',
    key: 'Escape',
    preventDefault: () => { prevented = true },
  })
  flushTimers()

  assert.equal(prevented, true)
  assert.deepEqual(calls, ['cancel'])
  assert.equal(region.children.length, 0)
  assert.equal(document.activeElement, trigger)
})

test('confirm toast runs the destructive action only from the primary button', () => {
  const { region, flushTimers } = setupToast()
  const calls = []

  showConfirmToast({
    message: 'Clear local browsing history from this Mac?',
    confirmLabel: 'Clear History',
    cancelLabel: 'Keep History',
    focusCancel: true,
    onConfirm: () => calls.push('confirm'),
    onCancel: () => calls.push('cancel'),
  })

  region.querySelector('.toast-action-primary').dispatchEvent({ type: 'click' })
  flushTimers()

  assert.deepEqual(calls, ['confirm'])
  assert.equal(region.children.length, 0)
})
