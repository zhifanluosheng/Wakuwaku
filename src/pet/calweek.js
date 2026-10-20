// The calendar's week view as arithmetic: where a block stands on the grid,
// what a drop or a grip or an arrow key does to it. Plain data in, plain
// data out, for settings.js to draw and the tests to try (test/calendar.test.js),
// as deskgrid.js is the desktop's layout.
//
// The grid: 24 rows, one an hour, HOUR_PX tall; a block's top is its start in
// minutes / 60 * HOUR_PX. A drop or a grip lands on the nearest quarter hour
// (SNAP), so nothing lands at 09:07. An event is { id, date: "YYYY-MM-DD",
// start: "HH:MM", end: "HH:MM" } as data/calendar.json keeps it; one with no
// start is an all-day event and has no place on this grid.
;(function (root) {
  const HOUR_PX = 44
  const SNAP = 15
  const MIN_DAY = 0
  const MAX_MIN = 24 * 60

  // "HH:MM" as minutes since midnight, or null when there is no time (an
  // all-day event, or a field that is not one).
  const min = hhmm => {
    const text = String(hhmm == null ? '' : hhmm).trim()
    // An empty field, or one that is not "HH:MM" at all, is no time: an
    // all-day event, which has no place on this grid. Number('') is 0, so
    // the shape has to be checked before the digits are read.
    if (!/^\d{1,2}(:\d{1,2})?$/.test(text)) return null
    const [h, m] = text.split(':').map(Number)
    return h * 60 + (m || 0)
  }
  // And back. Past midnight it holds at 24:00 rather than rolling into the
  // next day: an event has one day, and the day it lands on is the column it
  // was dropped into.
  const hhmm = m => {
    const v = Math.max(MIN_DAY, Math.min(MAX_MIN, Math.round(Number(m) || 0)))
    return `${String(Math.floor(v / 60)).padStart(2, '0')}:${String(v % 60).padStart(2, '0')}`
  }

  // Pixels down the column to minutes, and back. The drop and the grip read
  // the pointer through this, so they cannot disagree about where an hour is.
  const pxToMin = px => Math.round((Number(px) || 0) / HOUR_PX * 60 / SNAP) * SNAP
  const minToPx = m => ((Number(m) || 0) / 60) * HOUR_PX

  // How long an event lasts, in minutes, at least a quarter of an hour (one
  // with no end, or one whose end is before its start, still takes room to
  // be seen and grabbed).
  const lengthOf = e => {
    const from = min(e && e.start)
    if (from == null) return null
    const to = min(e && e.end)
    return Math.max(SNAP, (to == null ? from : to) - from)
  }

  // An event is the one with that id in that account: the same event may be
  // in two accounts (copied to both), and moving one must not move the
  // other's. An event with no account of its own is its own.
  const sameEvent = (e, id, account) => String(e.id) === String(id) && String(e.account || '') === String(account || '')

  // Move an event to (day, minute): it keeps how long it lasts, and nothing
  // else about it changes. The whole new list comes back, or null when there
  // is no such event or it has no time to sit on the grid.
  const move = (events, id, day, startMin, account) => {
    const list = Array.isArray(events) ? events : []
    const e = list.find(x => sameEvent(x, id, account))
    const length = lengthOf(e)
    if (!e || length == null) return null
    const next = { ...e, date: day, start: hhmm(startMin), end: hhmm(startMin + length) }
    return list.map(x => (sameEvent(x, id, account) ? next : x))
  }

  // Change only where an event ends: the start stays where it was, and the
  // grip may not pull the end above the start (it stops a quarter of an hour
  // after it).
  const resize = (events, id, endMin, account) => {
    const list = Array.isArray(events) ? events : []
    const e = list.find(x => sameEvent(x, id, account))
    const from = min(e && e.start)
    if (from == null) return null
    const end = Math.max(from + SNAP, Number(endMin) || 0)
    const next = { ...e, end: hhmm(end) }
    return list.map(x => (sameEvent(x, id, account) ? next : x))
  }

  const api = { HOUR_PX, SNAP, min, hhmm, pxToMin, minToPx, lengthOf, sameEvent, move, resize }
  if (typeof module !== 'undefined' && module.exports) module.exports = api
  root.CalWeek = api
})(typeof globalThis !== 'undefined' ? globalThis : this)