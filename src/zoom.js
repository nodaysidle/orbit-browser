const DEFAULT_ZOOM_LEVEL = 1.0
const MIN_ZOOM_LEVEL = 0.5
const MAX_ZOOM_LEVEL = 3.0
const ZOOM_STEP = 0.1

export function createZoomController({ state, command, getSetting, saveSetting, absorb, logError, showToast, isWebUrl }) {
  function getOrigin(url) {
    try {
      return new URL(url).origin
    } catch {
      return null
    }
  }

  function clampZoomLevel(value) {
    if (!Number.isFinite(value)) return DEFAULT_ZOOM_LEVEL
    const rounded = Math.round(value * 10) / 10
    return Math.min(MAX_ZOOM_LEVEL, Math.max(MIN_ZOOM_LEVEL, rounded))
  }

  async function loadZoomMemory() {
    try {
      const raw = await getSetting('zoom_memory')
      if (raw) {
        const parsed = JSON.parse(raw)
        state.zoomMemory = new Map(Object.entries(parsed))
      }
    } catch (e) {
      logError('Failed to load zoom memory', e)
      showToast('Could not restore zoom preferences', 'error')
    }
  }

  async function saveZoomMemory() {
    try {
      const obj = Object.fromEntries(state.zoomMemory)
      await saveSetting('zoom_memory', JSON.stringify(obj), 'Could not save zoom memory')
    } catch (e) {
      logError('Failed to save zoom memory', e)
    }
  }

  function savedZoomForUrl(url) {
    const origin = getOrigin(url)
    if (!origin) return DEFAULT_ZOOM_LEVEL
    return clampZoomLevel(state.zoomMemory.get(origin) ?? DEFAULT_ZOOM_LEVEL)
  }

  async function persistZoomForUrl(url, zoomLevel) {
    const origin = getOrigin(url)
    if (!origin) return
    state.zoomMemory.set(origin, clampZoomLevel(zoomLevel))
    await saveZoomMemory()
  }

  function currentZoomLevelForTab(tabId = state.activeId) {
    const tab = state.tabs.get(tabId)
    if (!tab?.url) return DEFAULT_ZOOM_LEVEL
    return clampZoomLevel(state.tabZoomLevels.get(tabId) ?? savedZoomForUrl(tab.url))
  }

  async function setTabZoomLevel(tabId, zoomLevel) {
    const tab = state.tabs.get(tabId)
    if (!tab?.id || !isWebUrl(tab.url)) return DEFAULT_ZOOM_LEVEL
    const applied = await command(
      'set_tab_zoom',
      { tabId, zoomLevel: clampZoomLevel(zoomLevel) },
      'Could not update zoom'
    )
    state.tabZoomLevels.set(tabId, applied)
    await persistZoomForUrl(tab.url, applied)
    return applied
  }

  function applySavedZoomToTab(tabId) {
    const tab = state.tabs.get(tabId)
    if (!tab?.id || !isWebUrl(tab.url)) return Promise.resolve(DEFAULT_ZOOM_LEVEL)
    return setTabZoomLevel(tabId, savedZoomForUrl(tab.url))
  }

  function zoomIn() {
    return setTabZoomLevel(state.activeId, currentZoomLevelForTab() + ZOOM_STEP)
  }

  function zoomOut() {
    return setTabZoomLevel(state.activeId, currentZoomLevelForTab() - ZOOM_STEP)
  }

  function resetZoom() {
    return setTabZoomLevel(state.activeId, DEFAULT_ZOOM_LEVEL)
  }

  function syncTabEnhancements(tabId) {
    return applySavedZoomToTab(tabId)
  }

  function clearTabEnhancements(tabId) {
    state.tabZoomLevels.delete(tabId)
  }

  return {
    loadZoomMemory,
    currentZoomLevelForTab,
    setTabZoomLevel,
    applySavedZoomToTab,
    zoomIn,
    zoomOut,
    resetZoom,
    syncTabEnhancements,
    clearTabEnhancements,
  }
}
