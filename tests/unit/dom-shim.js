export function installDom() {
  const document = new FakeDocument()
  const windowListeners = new Map()
  globalThis.document = document
  globalThis.window = {
    document,
    listeners: windowListeners,
    addEventListener: (type, listener) => addListener(windowListeners, type, listener),
    dispatchEvent: event => dispatch(globalThis.window, event),
    setTimeout: (callback) => {
      callback()
      return 1
    },
    clearTimeout: () => {},
    requestAnimationFrame: (callback) => {
      callback()
      return 1
    },
    cancelAnimationFrame: () => {},
    matchMedia: () => ({ matches: true, addEventListener: () => {}, removeEventListener: () => {} }),
    __TAURI_INTERNALS__: { metadata: { currentWindow: { label: 'main' } } },
  }
  globalThis.requestAnimationFrame = globalThis.window.requestAnimationFrame
  globalThis.cancelAnimationFrame = globalThis.window.cancelAnimationFrame
  globalThis.getComputedStyle = () => ({ getPropertyValue: () => '124px' })
  return document
}

export class FakeDocument {
  constructor() {
    this.body = new FakeElement('body')
    this.activeElement = null
    this.listeners = new Map()
  }

  createElement(tagName) {
    return new FakeElement(tagName)
  }

  createElementNS(namespaceURI, tagName) {
    return new FakeElement(tagName, namespaceURI)
  }

  addEventListener(type, listener) {
    addListener(this.listeners, type, listener)
  }

  dispatchEvent(event) {
    dispatch(this, event)
  }

  querySelector(selector) {
    return this.body.querySelector(selector)
  }

  querySelectorAll(selector) {
    return this.body.querySelectorAll(selector)
  }

  getElementById(id) {
    return this.body.querySelector(`#${id}`)
  }
}

export class FakeElement {
  constructor(tagName, namespaceURI = null) {
    this.tagName = tagName.toUpperCase()
    this.namespaceURI = namespaceURI
    this.children = []
    this.parentElement = null
    this.dataset = {}
    this.attributes = new Map()
    this.listeners = new Map()
    this.style = {}
    this.className = ''
    this.textContent = ''
    this.value = ''
    this.type = ''
    this.title = ''
    this.id = ''
    this.disabled = false
    this.classList = {
      add: (...names) => {
        const next = new Set(classNames(this))
        names.forEach(name => next.add(name))
        this.className = [...next].join(' ')
      },
      remove: (...names) => {
        const remove = new Set(names)
        this.className = classNames(this).filter(name => !remove.has(name)).join(' ')
      },
      contains: name => classNames(this).includes(name),
      toggle: (name, force) => {
        const has = this.classList.contains(name)
        const shouldAdd = force ?? !has
        if (shouldAdd) this.classList.add(name)
        else this.classList.remove(name)
        return shouldAdd
      },
    }
  }

  setAttribute(key, value) {
    const text = String(value)
    this.attributes.set(key, text)
    if (key === 'class') this.className = text
    if (key === 'id') this.id = text
    if (key === 'value') this.value = text
    if (key.startsWith('data-')) this.dataset[dataKey(key)] = text
  }

  getAttribute(key) {
    return this.attributes.get(key) ?? null
  }

  append(...children) {
    children.filter(Boolean).forEach(child => {
      child.parentElement = this
      this.children.push(child)
    })
  }

  replaceChildren(...children) {
    this.children.forEach(child => {
      child.parentElement = null
    })
    this.children = []
    this.append(...children)
  }

  remove() {
    if (!this.parentElement) return
    this.parentElement.children = this.parentElement.children.filter(child => child !== this)
    this.parentElement = null
  }

  addEventListener(type, listener) {
    addListener(this.listeners, type, listener)
  }

  dispatchEvent(event) {
    event.target ??= this
    event.currentTarget = this
    dispatch(this, event)
  }

  focus() {
    globalThis.document.activeElement = this
  }

  blur() {
    if (globalThis.document.activeElement === this) globalThis.document.activeElement = null
  }

  select() {
    this.selected = true
  }

  querySelector(selector) {
    return this.querySelectorAll(selector)[0] || null
  }

  querySelectorAll(selector) {
    const matches = []
    walk(this, node => {
      if (node !== this && node.matches(selector)) matches.push(node)
    })
    return matches
  }

  closest(selector) {
    let node = this
    while (node) {
      if (node.matches(selector)) return node
      node = node.parentElement
    }
    return null
  }

  matches(selector) {
    if (selector.startsWith('#')) return this.id === selector.slice(1)
    if (selector.startsWith('.')) return this.classList.contains(selector.slice(1))
    const dataMatch = selector.match(/^\[data-([a-z0-9-]+)\]$/i)
    if (dataMatch) return this.dataset[dataKey(`data-${dataMatch[1]}`)] !== undefined
    return this.tagName.toLowerCase() === selector.toLowerCase()
  }
}

function addListener(listeners, type, listener) {
  if (!listeners.has(type)) listeners.set(type, [])
  listeners.get(type).push(listener)
}

function dispatch(target, event) {
  for (const listener of target.listeners.get(event.type) || []) {
    listener(event)
  }
}

function walk(node, visitor) {
  visitor(node)
  node.children.forEach(child => walk(child, visitor))
}

function classNames(node) {
  return String(node.className || '').split(/\s+/).filter(Boolean)
}

function dataKey(attribute) {
  return attribute
    .replace(/^data-/, '')
    .replace(/-([a-z])/g, (_, char) => char.toUpperCase())
}
