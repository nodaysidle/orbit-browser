import { el } from './dom.js'
import { formatError } from './ui.js'

const TOAST_DURATION_MS = 3200
const TOAST_FADE_MS = 190
const CONFIRM_TOAST_TIMEOUT_MS = 12000

export function logError(...args) {
  console.error(...args)
}

export function showToast(message, tone = 'error') {
  const region = document.getElementById('toastRegion')
  if (!region || !message) return

  const toast = el('div', { className: `toast toast-${tone}`, text: message })
  region.append(toast)
  requestAnimationFrame(() => toast.classList.add('visible'))
  window.setTimeout(() => {
    toast.classList.remove('visible')
    window.setTimeout(() => toast.remove(), TOAST_FADE_MS)
  }, TOAST_DURATION_MS)
}

export function showConfirmToast({ message, confirmLabel = 'Download', cancelLabel = 'Cancel', onConfirm, onCancel }) {
  const region = document.getElementById('toastRegion')
  if (!region || !message) return

  const toast = el('div', { className: 'toast toast-info toast-actions-wrap' })
  const content = el('div', { className: 'toast-message', text: message })
  const confirm = el('button', { className: 'toast-action toast-action-primary', type: 'button', text: confirmLabel })
  const cancel = el('button', { className: 'toast-action', type: 'button', text: cancelLabel })
  const actions = el('div', { className: 'toast-actions' }, [confirm, cancel])
  toast.replaceChildren(content, actions)
  region.append(toast)
  requestAnimationFrame(() => toast.classList.add('visible'))

  const close = () => {
    toast.classList.remove('visible')
    window.setTimeout(() => toast.remove(), TOAST_FADE_MS)
  }

  confirm.addEventListener('click', () => {
    try { onConfirm?.() } finally { close() }
  })
  cancel.addEventListener('click', () => {
    try { onCancel?.() } finally { close() }
  })

  window.setTimeout(() => {
    if (toast.isConnected) close()
  }, CONFIRM_TOAST_TIMEOUT_MS)
}

export function installUnhandledRejectionHandler() {
  window.addEventListener('unhandledrejection', event => {
    logError('Unhandled promise rejection', event.reason)
    showToast(formatError(event.reason, 'Unexpected Orbit error'))
  })
}
