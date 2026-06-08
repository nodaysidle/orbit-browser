const MODAL_FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'

let activeModal = null
let activeModalTrigger = null
let activeModalFocusHandler = null

function collectFocusables(modal) {
  return Array.from(modal.querySelectorAll(MODAL_FOCUSABLE_SELECTOR)).filter(
    element => element.tabIndex >= 0 && !element.hasAttribute('disabled')
  )
}

function bindModalFocusTrap(modal) {
  const focusables = collectFocusables(modal)
  if (!focusables.length) return
  const first = focusables[0]
  const last = focusables[focusables.length - 1]

  if (!focusables.includes(document.activeElement)) {
    first.focus()
  }

  activeModalFocusHandler = event => {
    if (event.key !== 'Tab' || focusables.length === 0) return
    if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault()
      first.focus()
    } else if (event.shiftKey && document.activeElement === first) {
      event.preventDefault()
      last.focus()
    }
  }

  modal.addEventListener('keydown', activeModalFocusHandler)
}

export function closeModal(modal, fallbackFocus = null) {
  if (!modal) return
  modal.classList.add('hidden')
  if (activeModalFocusHandler) {
    modal.removeEventListener('keydown', activeModalFocusHandler)
  }
  activeModalFocusHandler = null
  activeModal = null
  if (activeModalTrigger instanceof HTMLElement) {
    activeModalTrigger.focus()
  } else {
    fallbackFocus?.focus?.()
  }
  activeModalTrigger = null
}

export function openModal(modal, fallbackSelector) {
  if (!modal) return
  activeModal = modal
  activeModalTrigger = document.activeElement instanceof HTMLElement ? document.activeElement : null
  modal.classList.remove('hidden')
  const fallback = fallbackSelector ? modal.querySelector(fallbackSelector) : null
  if (fallback instanceof HTMLElement) fallback.focus()
  bindModalFocusTrap(modal)
}
