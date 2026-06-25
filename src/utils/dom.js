const SVG_NS = 'http://www.w3.org/2000/svg'

const ICON_PATHS = {
  back: [['path', { d: 'M10 14 6 10l4-4' }]],
  bookmark: [['path', { d: 'M6 4h8a1 1 0 0 1 1 1v11l-5-3-5 3V5a1 1 0 0 1 1-1Z' }]],
  chevronUp: [['path', { d: 'm6 12 4-4 4 4' }]],
  clock: [
    ['circle', { cx: '10', cy: '10', r: '6' }],
    ['path', { d: 'M10 6.8V10l2.3 1.4' }],
  ],
  close: [['path', { d: 'm6 6 8 8M14 6l-8 8' }]],
  down: [['path', { d: 'm6 8 4 4 4-4' }]],
  error: [
    ['path', { d: 'M10 3.5 18 17.5H2L10 3.5Z' }],
    ['path', { d: 'M10 8.5v4' }],
    ['circle', { cx: '10', cy: '15', r: '0.6', fill: 'currentColor' }],
  ],
  forward: [['path', { d: 'm8 6 4 4-4 4' }]],
  lock: [
    ['rect', { x: '5', y: '9', width: '10', height: '7', rx: '2' }],
    ['path', { d: 'M7 9V7a3 3 0 0 1 6 0v2' }],
  ],
  menu: [
    ['circle', { cx: '10', cy: '5', r: '1' }],
    ['circle', { cx: '10', cy: '10', r: '1' }],
    ['circle', { cx: '10', cy: '15', r: '1' }],
  ],
  moon: [['path', { d: 'M15 13.5A6.5 6.5 0 0 1 6.5 5 6.8 6.8 0 1 0 15 13.5Z' }]],
  orbit: [
    ['circle', { cx: '10', cy: '10', r: '7', fill: 'none', stroke: 'currentColor', 'stroke-width': '1.4' }],
    ['circle', { cx: '10', cy: '10', r: '2.5', fill: 'currentColor' }],
  ],
  plus: [['path', { d: 'M10 4v12M4 10h12' }]],
  reload: [
    ['path', { d: 'M15 6v4h-4' }],
    ['path', { d: 'M5 14v-4h4' }],
    ['path', { d: 'M14.5 10A5 5 0 0 0 6 6.5L5 8m.5 2A5 5 0 0 0 14 13.5l1-1.5' }],
  ],
  search: [['path', { d: 'm14 14 3 3M15 9.5A5.5 5.5 0 1 1 4 9.5a5.5 5.5 0 0 1 11 0Z' }]],
  settings: [
    ['circle', { cx: '10', cy: '10', r: '2.4' }],
    ['path', { d: 'M10 3.5v2M10 14.5v2M15.6 6.8l-1.7 1M6.1 12.2l-1.7 1M15.6 13.2l-1.7-1M6.1 7.8l-1.7-1' }],
  ],
  sparkle: [['path', { d: 'M10 3.5 11.8 8l4.7 2-4.7 2L10 16.5 8.2 12l-4.7-2 4.7-2Z' }]],
  star: [['path', { d: 'm10 3.8 1.8 3.7 4.1.6-3 2.9.7 4.1-3.6-1.9-3.6 1.9.7-4.1-3-2.9 4.1-.6Z' }]],
  trash: [
    ['path', { d: 'M5 6h10' }],
    ['path', { d: 'M8 6V4h4v2' }],
    ['path', { d: 'M7 8v7h6V8' }],
  ],
  sun: [
    ['circle', { cx: '10', cy: '10', r: '3' }],
    ['path', { d: 'M10 2.5v2M10 15.5v2M17.5 10h-2M4.5 10h-2M15.3 4.7l-1.4 1.4M6.1 13.9l-1.4 1.4M15.3 15.3l-1.4-1.4M6.1 6.1 4.7 4.7' }],
  ],
  system: [
    ['rect', { x: '4', y: '5', width: '12', height: '8', rx: '1.5' }],
    ['path', { d: 'M8 16h4M10 13v3' }],
  ],
}

export function el(tagName, options = {}, children = []) {
  const node = document.createElement(tagName)
  applyOptions(node, options)
  if (children.length > 0) {
    node.replaceChildren(...children.filter(Boolean))
  }
  return node
}

export function icon(name, size = 18) {
  const svg = document.createElementNS(SVG_NS, 'svg')
  svg.setAttribute('viewBox', '0 0 20 20')
  svg.setAttribute('width', String(size))
  svg.setAttribute('height', String(size))
  svg.setAttribute('aria-hidden', 'true')
  svg.setAttribute('focusable', 'false')
  svg.classList.add('icon')

  for (const [tagName, attrs] of ICON_PATHS[name] || []) {
    const child = document.createElementNS(SVG_NS, tagName)
    for (const [key, value] of Object.entries(attrs)) {
      child.setAttribute(key, value)
    }
    svg.append(child)
  }

  return svg
}

function applyOptions(node, options) {
  if (options.className) node.className = options.className
  if (options.text !== undefined) node.textContent = options.text
  if (options.type) node.type = options.type
  if (options.title) node.title = options.title
  if (options.id) node.id = options.id
  if (options.dataset) {
    for (const [key, value] of Object.entries(options.dataset)) {
      node.dataset[key] = value
    }
  }
  if (options.attrs) {
    for (const [key, value] of Object.entries(options.attrs)) {
      if (value !== false && value !== null && value !== undefined) {
        node.setAttribute(key, String(value))
      }
    }
  }
}
