export function createFindController({ $, state, command, absorb, updateOverlay }) {
  function focusFindInput({ select = false } = {}) {
    const input = $('findInput')
    input.focus()
    if (select) input.select()
  }

  function setFindStatus(text) {
    const status = $('findStatus')
    if (status) status.textContent = text
  }

  function openFindBar() {
    $('findBar').classList.remove('hidden')
    setFindStatus('')
    focusFindInput({ select: true })
    updateOverlay()
  }

  function closeFindBar() {
    $('findBar').classList.add('hidden')
    setFindStatus('')
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
    setFindStatus('...')
    return command('find_in_page', { tabId: state.activeId, query, backwards: false })
      .then(() => setFindStatus(''))
      .catch(() => setFindStatus('!'))
  }

  function findPrev() {
    const query = $('findInput').value.trim()
    focusFindInput()
    if (!query) return Promise.resolve()
    setFindStatus('...')
    return command('find_in_page', { tabId: state.activeId, query, backwards: true })
      .then(() => setFindStatus(''))
      .catch(() => setFindStatus('!'))
  }

  return { openFindBar, closeFindBar, findNext, findPrev }
}
