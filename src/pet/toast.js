// The bottom-right notices (toast.rs): a calendar event due, or a new
// letter with a verification code. They queue here, one at a time, nine
// seconds each. The window is click-through, so nothing waits on them.
const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

const root = document.getElementById('toast')
const queue = []
let showing = false
let timer = 0
let lastSeq = 0

const WORDS = {
  zh: { calendar: '日程提醒', date: '日期', event: '事件', code: '验证码', allDay: '全天' },
  en: { calendar: 'Calendar reminder', date: 'Date', event: 'Event', code: 'Verification code', allDay: 'All day' },
}

function words(lang) {
  return WORDS[lang === 'zh' ? 'zh' : 'en']
}

function el(tag, cls, text) {
  const node = document.createElement(tag)
  if (cls) node.className = cls
  if (text !== undefined && text !== null) node.textContent = String(text)
  return node
}

function calendarIcon() {
  const box = el('div', 'cal-icon')
  box.innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3.5" y="5" width="17" height="15.5" rx="3"></rect><path d="M7.5 3.5v3M16.5 3.5v3M3.5 9.5h17"></path><path d="M8 13h2M14 13h2M8 16.5h2M14 16.5h2"></path></svg>'
  return box
}

function calendarDate(p) {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(String(p.date || ''))
  const locale = p.lang === 'zh' ? 'zh-CN' : 'en-US'
  let text = String(p.date || '')
  if (m) {
    const day = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]))
    if (!Number.isNaN(day.getTime())) {
      text = day.toLocaleDateString(locale, { year: 'numeric', month: 'long', day: 'numeric', weekday: 'short' })
    }
  }
  if (p.allDay) text += ` · ${words(p.lang).allDay}`
  else if (p.start) text += ` ${p.start}`
  return text
}

function renderCalendar(p) {
  const t = words(p.lang)
  const wrap = el('div', 'cal')
  wrap.append(calendarIcon())
  const body = el('div', 'cal-body')
  body.append(el('div', 'eyebrow', t.calendar))
  body.append(el('div', 'cal-date', calendarDate(p)))
  body.append(el('div', 'cal-title', p.title || ''))
  wrap.append(body)
  return wrap
}

function hueOf(text) {
  let hash = 0
  for (const ch of String(text || '')) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0
  return hash % 360
}

function renderMail(p) {
  const t = words(p.lang)
  const wrap = el('div', 'mail')
  const name = String(p.name || p.address || '').trim()
  const address = String(p.address || '').trim()
  const avatar = el('div', 'avatar', Array.from(name || address || '?')[0].toLocaleUpperCase())
  avatar.style.setProperty('--av-h', String(hueOf(address || name)))
  const who = el('div', 'who')
  who.append(el('div', 'eyebrow', t.code))
  if (name && name !== address) who.append(el('div', 'who-name', name))
  if (address) who.append(el('div', 'who-addr', address))
  const box = el('div', 'codebox')
  box.append(el('div', 'code-label', t.code))
  box.append(el('div', 'code', p.code || ''))
  wrap.append(avatar, who, box)
  return wrap
}

function render(p) {
  root.replaceChildren()
  root.style.setProperty('--accent', p.kind === 'mail' ? '#7fb0ff' : '#5e9bff')
  root.append(p.kind === 'mail' ? renderMail(p) : renderCalendar(p))
  root.classList.remove('leaving')
  // A frame between putting it in and showing it, so the entrance runs.
  requestAnimationFrame(() => root.classList.add('show'))
}

function finish() {
  root.classList.remove('show')
  root.classList.add('leaving')
  timer = window.setTimeout(() => {
    root.replaceChildren()
    root.classList.remove('leaving')
    showing = false
    if (queue.length) pump()
    else invoke('toast_hide', { seq: lastSeq }).catch(() => {})
  }, 240)
}

function pump() {
  if (showing || !queue.length) return
  showing = true
  render(queue.shift())
  window.clearTimeout(timer)
  timer = window.setTimeout(finish, 9000)
}

listen('toast:show', event => {
  const payload = event && event.payload ? event.payload : event
  if (!payload || (payload.kind !== 'calendar' && payload.kind !== 'mail')) return
  if (Number.isFinite(Number(payload.seq))) lastSeq = Number(payload.seq)
  queue.push(payload)
  pump()
})

// The page is listening: whatever waited in Rust comes now.
invoke('toast_ready').catch(() => {})

// The debug endpoint (WAKUWAKU_DEBUG=1) can look in here, as in her pages.
window.__wkEval = (id, fn) => {
  Promise.resolve()
    .then(fn)
    .catch(err => 'error: ' + err.message)
    .then(value => invoke('debug_result', { id, value: value === undefined ? null : value }))
}
