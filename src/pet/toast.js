// The bottom-right notices (toast.rs): a calendar event due, or a new
// letter with a verification code. They queue here, one at a time, nine
// seconds each. Calendar notices pass clicks through; a code notice takes
// the mouse so its code can be copied with one click.
const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

const root = document.getElementById('toast')
const queue = []
let showing = false
let timer = 0
let lastSeq = 0

const WORDS = {
  zh: {
    calendar: '日历提醒 · 日程',
    calendarBadge: '已提醒',
    calendarSource: 'Wakuwaku 日历',
    delivered: '提醒已送达',
    mail: '邮件提醒 · 新邮件',
    mailBadge: '验证码',
    mailSource: 'Wakuwaku Mail',
    codeFound: '验证码已识别',
    justArrived: '刚刚收到',
    copy: '点击复制验证码',
    copied: '已复制',
    copyFailed: '复制失败',
    untitled: '未命名日程',
    noSubject: '新邮件中的验证码',
    allDay: '全天',
    leadSame: '当天提醒',
    leadDays: n => `提前 ${n} 天`,
    leadMinutes: n => `提前 ${n} 分钟`,
    repeatNone: '不重复',
    repeatDaily: '每天',
    repeatWeekly: '每周',
    repeatMonthly: '每月',
  },
  en: {
    calendar: 'Calendar · Event',
    calendarBadge: 'Reminder',
    calendarSource: 'Wakuwaku Calendar',
    delivered: 'Reminder delivered',
    mail: 'Mail · New message',
    mailBadge: 'Code',
    mailSource: 'Wakuwaku Mail',
    codeFound: 'Verification code found',
    justArrived: 'Just arrived',
    copy: 'Click to copy code',
    copied: 'Copied',
    copyFailed: 'Copy failed',
    untitled: 'Untitled event',
    noSubject: 'Verification code in a new message',
    allDay: 'All day',
    leadSame: 'On time',
    leadDays: n => `${n} ${n === 1 ? 'day' : 'days'} before`,
    leadMinutes: n => `${n} ${n === 1 ? 'minute' : 'minutes'} before`,
    repeatNone: 'Does not repeat',
    repeatDaily: 'Every day',
    repeatWeekly: 'Every week',
    repeatMonthly: 'Every month',
  },
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
  const box = el('span', 'well-icon')
  box.innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3.5" y="5" width="17" height="15.5" rx="3"></rect><path d="M7.5 3.5v3M16.5 3.5v3M3.5 9.5h17"></path><path d="M8 13h2M14 13h2M8 16.5h2M14 16.5h2"></path></svg>'
  return box
}

function calendarDay(p) {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(String(p.date || ''))
  const locale = p.lang === 'zh' ? 'zh-CN' : 'en-US'
  if (!m) return String(p.date || '')
  const day = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]))
  if (Number.isNaN(day.getTime())) return String(p.date || '')
  return day.toLocaleDateString(locale, { year: 'numeric', month: 'long', day: 'numeric', weekday: 'short' })
}

function calendarTime(p, t) {
  if (p.allDay) return t.allDay
  if (!p.start) return ''
  if (p.end && p.end !== p.start) return `${p.start}–${p.end}`
  return String(p.start)
}

function calendarDescription(p, t) {
  const when = [calendarDay(p), calendarTime(p, t)].filter(Boolean).join(' · ')
  return [when, String(p.where || '').trim()].filter(Boolean).join(' · ')
}

function calendarLead(p, t) {
  if (p.remindDays !== null && p.remindDays !== undefined && Number.isFinite(Number(p.remindDays))) {
    const days = Math.max(0, Number(p.remindDays))
    return days === 0 ? t.leadSame : t.leadDays(days)
  }
  if (Number(p.remind) > 0) return t.leadMinutes(Number(p.remind))
  return t.delivered
}

function repeatLabel(p, t) {
  switch (String(p.repeat || 'none')) {
    case 'daily':
      return t.repeatDaily
    case 'weekly':
      return t.repeatWeekly
    case 'monthly':
      return t.repeatMonthly
    default:
      return t.repeatNone
  }
}

function metaRow(lines) {
  const meta = el('div', 'product-meta mono')
  for (const line of lines) {
    if (line) meta.append(el('span', '', line))
  }
  return meta
}

function renderCalendar(p) {
  const t = words(p.lang)
  const card = el('article', 'card product')
  const top = el('div', 'product-top')
  top.append(calendarIcon(), el('span', 'badge-ok', t.calendarBadge))

  const copy = el('div', 'product-copy')
  copy.append(el('span', 'product-cat mono', t.calendar))
  copy.append(el('h4', 'product-title', String(p.title || '').trim() || t.untitled))
  copy.append(el('p', 'product-desc', calendarDescription(p, t)))

  const rule = el('div', 'rule')
  rule.setAttribute('aria-hidden', 'true')

  const links = el('div', 'product-links')
  links.append(el('span', 'product-link', t.calendarSource))
  links.append(el('span', 'product-link product-link-end', t.delivered))

  card.append(top, copy, metaRow([calendarLead(p, t), repeatLabel(p, t)]), rule, links)
  return card
}

function hueOf(text) {
  let hash = 0
  for (const ch of String(text || '')) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0
  return hash % 360
}

function copyIcon() {
  const box = el('span', 'copy-icon')
  box.innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="8.5" y="8.5" width="11" height="11" rx="2.5"></rect><path d="M15.5 8.5V6.5a2 2 0 0 0-2-2h-7a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h2"></path></svg>'
  return box
}

async function copyText(text) {
  try {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(text)
      return true
    }
  } catch {
    // A window that never takes the focus may be refused this; try below.
  }
  try {
    if (await invoke('toast_copy', { text })) return true
  } catch {
    // Not Windows, or the clipboard was busy: use the document's copy.
  }
  try {
    const area = document.createElement('textarea')
    area.value = text
    area.setAttribute('readonly', '')
    area.style.position = 'fixed'
    area.style.left = '-9999px'
    area.style.top = '0'
    document.body.append(area)
    area.select()
    const copied = document.execCommand('copy')
    area.remove()
    return copied
  } catch {
    return false
  }
}

function codeButton(p, t) {
  const button = el('button', 'code-copy')
  button.type = 'button'
  button.setAttribute('aria-label', t.copy)
  const hint = el('span', 'copy-hint', t.copy)
  button.append(el('span', 'code', p.code || ''), hint, copyIcon())

  button.addEventListener('click', async event => {
    event.stopPropagation()
    const copied = await copyText(String(p.code || ''))
    window.clearTimeout(button._reset)
    button.classList.toggle('copied', copied)
    button.classList.toggle('failed', !copied)
    hint.textContent = copied ? t.copied : t.copyFailed
    if (copied) holdBriefly()
    button._reset = window.setTimeout(() => {
      button.classList.remove('copied', 'failed')
      hint.textContent = t.copy
    }, 1800)
  })
  return button
}

function renderMail(p) {
  const t = words(p.lang)
  const name = String(p.name || p.address || '').trim()
  const address = String(p.address || '').trim()

  const card = el('article', 'card product product-mail')
  const top = el('div', 'product-top')
  const avatar = el('span', 'well-icon avatar', Array.from(name || address || '?')[0].toLocaleUpperCase())
  avatar.style.setProperty('--av-h', String(hueOf(address || name)))
  top.append(avatar, el('span', 'badge-code', t.mailBadge))

  const copy = el('div', 'product-copy')
  copy.append(el('span', 'product-cat mono', t.mail))
  copy.append(el('h4', 'product-title', name || t.noSubject))
  copy.append(el('p', 'product-desc', address && address !== name ? address : t.noSubject))

  const rule = el('div', 'rule')
  rule.setAttribute('aria-hidden', 'true')

  const links = el('div', 'product-links')
  if (p.code) links.append(codeButton(p, t))
  links.append(el('span', 'product-link product-link-end', t.mailSource))

  card.append(top, copy, metaRow([t.codeFound, t.justArrived]), rule, links)
  return card
}

function fit() {
  const card = root.querySelector('.card')
  if (!card) return
  const before = card.style.height
  card.style.height = 'auto'
  // The window's two border pixels are outside the card.
  const height = card.offsetHeight + 2
  card.style.height = before
  invoke('toast_fit', { height }).catch(() => {})
}

function render(p) {
  root.replaceChildren()
  root.style.setProperty('--accent', p.kind === 'mail' ? '#7fb0ff' : '#5e9bff')
  root.dataset.kind = p.kind === 'mail' ? 'mail' : 'calendar'
  root.append(p.kind === 'mail' ? renderMail(p) : renderCalendar(p))
  root.classList.remove('leaving')
  invoke('toast_interactive', { on: p.kind === 'mail' }).catch(() => {})
  // A frame between putting it in and showing it, so the entrance runs.
  requestAnimationFrame(() => {
    fit()
    root.classList.add('show')
  })
}

function holdBriefly() {
  if (!showing) return
  window.clearTimeout(timer)
  timer = window.setTimeout(finish, 2600)
}

function finish() {
  root.classList.remove('show')
  root.classList.add('leaving')
  timer = window.setTimeout(() => {
    root.replaceChildren()
    root.classList.remove('leaving')
    showing = false
    if (queue.length) {
      pump()
    } else {
      invoke('toast_interactive', { on: false }).catch(() => {})
      invoke('toast_hide', { seq: lastSeq }).catch(() => {})
    }
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
