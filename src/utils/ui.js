const TAB_TONE_COUNT = 5
const THEMES = new Set(['auto', 'dark', 'light'])
const SEARCH_ENGINES = new Set(['duckduckgo', 'google', 'brave'])
const HTTP_SCHEME_RE = /^https?:\/\//i
const EXPLICIT_SCHEME_RE = /^[a-z][a-z0-9+.-]*:/i
const EXPLICIT_SCHEME_WITH_SLASHES_RE = /^[a-z][a-z0-9+.-]*:\/\//i

export function getUrlHost(value = '') {
  try {
    return new URL(value).hostname.replace(/^www\./, '')
  } catch {
    return ''
  }
}

export function normalizeNavigationInput(value = '', searchEngine = 'duckduckgo') {
  const input = String(value).trim()
  if (!input) return ''
  if (HTTP_SCHEME_RE.test(input)) return input
  if (EXPLICIT_SCHEME_WITH_SLASHES_RE.test(input) || (EXPLICIT_SCHEME_RE.test(input) && !looksLikeHostOrLocalUrl(input))) {
    return searchUrl(input, searchEngine)
  }
  if (looksLikeHostOrLocalUrl(input)) return `https://${input}`
  return searchUrl(input, searchEngine)
}

export function normalizeSearchEngine(value = '') {
  return SEARCH_ENGINES.has(value) ? value : 'duckduckgo'
}

export function searchUrl(input = '', searchEngine = 'duckduckgo') {
  const encoded = encodeURIComponent(String(input).trim())
  switch (normalizeSearchEngine(searchEngine)) {
    case 'google':
      return `https://www.google.com/search?q=${encoded}`
    case 'brave':
      return `https://search.brave.com/search?q=${encoded}`
    default:
      return `https://duckduckgo.com/?q=${encoded}`
  }
}

function looksLikeHostOrLocalUrl(input) {
  return input
    && !input.includes(' ')
    && (input.includes('.') || input.startsWith('localhost') || input.startsWith('[::1]'))
}

export function getNavigationTitle(value = '') {
  const input = String(value).trim()
  if (!input) return 'New Tab'
  if (!isBrowserUrlInput(input)) return `Search: ${input}`
  return getUrlHost(normalizeNavigationInput(input)) || input.replace(/^www\./, '')
}

function isBrowserUrlInput(input) {
  if (HTTP_SCHEME_RE.test(input)) return true
  if (EXPLICIT_SCHEME_WITH_SLASHES_RE.test(input) || (EXPLICIT_SCHEME_RE.test(input) && !looksLikeHostOrLocalUrl(input))) {
    return false
  }
  return looksLikeHostOrLocalUrl(input)
}

export function getTabMonogram(tab = {}) {
  const source = getUrlHost(tab.url) || String(tab.title || '').trim() || 'Orbit'
  return source[0]?.toUpperCase() || 'O'
}

export function getTabToneClass(tab = {}) {
  const source = getUrlHost(tab.url) || String(tab.title || '').trim() || 'orbit'
  let hash = 0
  for (const char of source) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0
  }
  return `tone-${hash % TAB_TONE_COUNT}`
}

export function formatError(error, fallback = 'Something went wrong') {
  if (!error) return fallback
  if (typeof error === 'string' && error.trim()) return error
  if (typeof error.message === 'string' && error.message.trim()) return error.message
  return fallback
}

export function restoreNavigationSnapshot(tabs, tabId, snapshot) {
  const next = new Map(tabs)
  if (!snapshot?.id || !next.has(tabId)) return next
  next.set(tabId, { ...snapshot, loading: false })
  return next
}

export function normalizeTheme(value = '') {
  return THEMES.has(value) ? value : 'auto'
}

export function nextTheme(value = '') {
  const theme = normalizeTheme(value)
  if (theme === 'auto') return 'dark'
  return theme === 'dark' ? 'light' : 'auto'
}

export function themeIcon(value = '') {
  const theme = normalizeTheme(value)
  if (theme === 'auto') return 'system'
  return theme === 'dark' ? 'sun' : 'moon'
}

export function isEditableTarget(target) {
  if (!target) return false
  const tagName = target.tagName?.toLowerCase?.()
  return target.isContentEditable || tagName === 'input' || tagName === 'textarea' || tagName === 'select'
}
