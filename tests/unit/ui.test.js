import test from 'node:test'
import assert from 'node:assert/strict'

import {
  formatError,
  getNavigationTitle,
  getTabMonogram,
  getTabToneClass,
  getUrlHost,
  isEditableTarget,
  nextTheme,
  normalizeNavigationInput,
  normalizeSearchEngine,
  normalizeTheme,
  restoreNavigationSnapshot,
  searchUrl,
  themeIcon,
} from '../../src/utils/ui.js'

test('getUrlHost normalizes common browser URLs', () => {
  assert.equal(getUrlHost('https://www.example.com/path?q=1'), 'example.com')
  assert.equal(getUrlHost('https://docs.rust-lang.org/book/'), 'docs.rust-lang.org')
})

test('getUrlHost returns an empty string for invalid values', () => {
  assert.equal(getUrlHost('not a url'), '')
  assert.equal(getUrlHost(''), '')
})

test('getTabMonogram derives a stable single-letter label', () => {
  assert.equal(getTabMonogram({ url: 'https://github.com/NODAYSIDLE' }), 'G')
  assert.equal(getTabMonogram({ title: 'New Tab' }), 'N')
  assert.equal(getTabMonogram({}), 'O')
})

test('getTabToneClass stays in the supported tone range', () => {
  const toneClass = getTabToneClass({ url: 'https://producthunt.com' })
  assert.match(toneClass, /^tone-[0-4]$/)
})

test('normalizeNavigationInput mirrors browser address behavior', () => {
  assert.equal(normalizeNavigationInput('example.com'), 'https://example.com')
  assert.equal(normalizeNavigationInput('https://youtube.com'), 'https://youtube.com')
  assert.equal(normalizeNavigationInput('HTTP://localhost:5173/test'), 'HTTP://localhost:5173/test')
  assert.equal(normalizeNavigationInput('localhost:5173/test'), 'https://localhost:5173/test')
  assert.equal(
    normalizeNavigationInput('ftp://example.com/archive.zip'),
    'https://duckduckgo.com/?q=ftp%3A%2F%2Fexample.com%2Farchive.zip',
  )
  assert.equal(normalizeNavigationInput('rust browser'), 'https://duckduckgo.com/?q=rust%20browser')
})

test('search engine helpers support persisted search preferences', () => {
  assert.equal(normalizeSearchEngine('brave'), 'brave')
  assert.equal(normalizeSearchEngine('unexpected'), 'duckduckgo')
  assert.equal(searchUrl('rust browser', 'google'), 'https://www.google.com/search?q=rust%20browser')
  assert.equal(normalizeNavigationInput('rust browser', 'brave'), 'https://search.brave.com/search?q=rust%20browser')
})

test('getNavigationTitle gives tabs useful labels before load completes', () => {
  assert.equal(getNavigationTitle('example.com'), 'example.com')
  assert.equal(getNavigationTitle('https://www.youtube.com'), 'youtube.com')
  assert.equal(getNavigationTitle('localhost:5173/test'), 'localhost')
  assert.equal(getNavigationTitle('ftp://example.com/archive.zip'), 'Search: ftp://example.com/archive.zip')
  assert.equal(getNavigationTitle('rust browser'), 'Search: rust browser')
})

test('formatError prefers useful messages and falls back cleanly', () => {
  assert.equal(formatError('No webview found', 'Fallback'), 'No webview found')
  assert.equal(formatError(new Error('Boom'), 'Fallback'), 'Boom')
  assert.equal(formatError(null, 'Fallback'), 'Fallback')
})

test('blocked navigation rollback restores an existing committed tab', () => {
  const snapshot = {
    id: 't1',
    url: 'https://example.com',
    title: 'example.com',
    loading: false,
  }
  const tabs = new Map([
    ['t1', {
      id: 't1',
      url: 'https://doubleclick.net/ad',
      title: 'doubleclick.net',
      loading: true,
    }],
  ])

  const restored = restoreNavigationSnapshot(tabs, 't1', snapshot)

  assert.deepEqual(restored.get('t1'), snapshot)
})

test('blocked navigation rollback restores a blank committed tab', () => {
  const snapshot = {
    id: 't2',
    url: '',
    title: 'New Tab',
    loading: false,
  }
  const tabs = new Map([
    ['t2', {
      id: 't2',
      url: 'https://doubleclick.net/ad',
      title: 'doubleclick.net',
      loading: true,
    }],
  ])

  const restored = restoreNavigationSnapshot(tabs, 't2', snapshot)

  assert.deepEqual(restored.get('t2'), snapshot)
})

test('isEditableTarget recognizes form-like editing surfaces', () => {
  assert.equal(isEditableTarget({ isContentEditable: true }), true)
  assert.equal(isEditableTarget({ tagName: 'INPUT' }), true)
  assert.equal(isEditableTarget({ tagName: 'button' }), false)
  assert.equal(isEditableTarget(null), false)
})

test('theme helpers default to auto and cycle predictably', () => {
  assert.equal(normalizeTheme('auto'), 'auto')
  assert.equal(normalizeTheme('light'), 'light')
  assert.equal(normalizeTheme('unexpected'), 'auto')
  assert.equal(nextTheme('auto'), 'dark')
  assert.equal(nextTheme('dark'), 'light')
  assert.equal(nextTheme('light'), 'auto')
  assert.equal(themeIcon('auto'), 'system')
  assert.equal(themeIcon('dark'), 'sun')
  assert.equal(themeIcon('light'), 'moon')
})
