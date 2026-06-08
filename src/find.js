export function createFindController({ $, state, command, absorb, updateOverlay }) {
  function focusFindInput({ select = false } = {}) {
    const input = $('findInput')
    input.focus()
    if (select) input.select()
  }

  function openFindBar() {
    $('findBar').classList.remove('hidden')
    focusFindInput({ select: true })
    updateOverlay()
  }

  function closeFindBar() {
    $('findBar').classList.add('hidden')
    updateOverlay()
    const tab = state.tabs.get(state.activeId)
    if (!tab?.url || (!tab.url.startsWith('http://') && !tab.url.startsWith('https://'))) {
      return Promise.resolve()
    }
    return command('find_in_page', { tabId: state.activeId, query: '', backwards: false })
  }

  function findNext() {
    const query = $('findInput').value.trim()
    focusFindInput()
    if (!query) return Promise.resolve()
    return command('find_in_page', { tabId: state.activeId, query, backwards: false })
  }

  function findPrev() {
    const query = $('findInput').value.trim()
    focusFindInput()
    if (!query) return Promise.resolve()
    return command('find_in_page', { tabId: state.activeId, query, backwards: true })
  }

  return { openFindBar, closeFindBar, findNext, findPrev }
}
