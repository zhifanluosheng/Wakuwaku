// The week view's arithmetic, in src/pet/calweek.js: where a block stands on
// the grid, and what a drop, the grip at its bottom, or an arrow key does to
// it. settings.js draws and handles the pointer; this is the part that has to
// be right, and it is plain data in, plain data out.
const assert = require('node:assert/strict')
const { test } = require('node:test')

const W = require('../src/pet/calweek')

// An event as data/calendar.json keeps it.
const event = (over = {}) => ({ id: 'a', date: '2026-10-20', start: '09:00', end: '10:30', title: '周会', ...over })

test('minutes to and from a clock time', () => {
  assert.equal(W.min('00:00'), 0)
  assert.equal(W.min('09:30'), 570)
  assert.equal(W.min('23:59'), 1439)
  assert.equal(W.min(''), null, 'no time: an all-day event is not on the grid')
  assert.equal(W.min(undefined), null)
  assert.equal(W.min('nonsense'), null)
  assert.equal(W.hhmm(0), '00:00')
  assert.equal(W.hhmm(570), '09:30')
  assert.equal(W.hhmm(1439), '23:59')
  // Past midnight it holds at 24:00 rather than rolling into another day:
  // an event has one day, and the day it lands on is the column it was
  // dropped into.
  assert.equal(W.hhmm(1500), '24:00')
  assert.equal(W.hhmm(-30), '00:00')
  // And back again, both ways.
  for (const t of ['00:00', '07:15', '09:30', '23:45']) assert.equal(W.hhmm(W.min(t)), t)
})

test('the grid is 44px an hour and a drop lands on the quarter hour', () => {
  assert.equal(W.HOUR_PX, 44)
  assert.equal(W.SNAP, 15)
  // An hour down the column is an hour later.
  assert.equal(W.minToPx(60), 44)
  assert.equal(W.pxToMin(44), 60)
  // Anywhere inside a quarter-hour band snaps to that quarter. A quarter of
  // an hour is 11 px of the 44, so the band ends just past 11.
  assert.equal(W.pxToMin(0), 0)
  assert.equal(W.pxToMin(5), 0)
  assert.equal(W.pxToMin(10), 15)
  assert.equal(W.pxToMin(16), 15)
  assert.equal(W.pxToMin(17), 30)
  assert.equal(W.pxToMin(44), 60)
  // A round trip through pixels keeps the quarter hour, so a block drawn
  // where the grid says stands there when it is read back.
  for (const m of [0, 15, 90, 570, 720]) assert.equal(W.pxToMin(W.minToPx(m)), m)
  // 09:00 is nine hours down the column: 9 * 44.
  assert.equal(W.minToPx(W.min('09:00')), 396)
})

test('how long an event lasts, at least a quarter of an hour', () => {
  assert.equal(W.lengthOf(event()), 90)
  assert.equal(W.lengthOf(event({ end: '' })), 15, 'no end: it takes room to be seen')
  assert.equal(W.lengthOf(event({ start: '09:00', end: '08:00' })), 15, 'an end before its start')
  assert.equal(W.lengthOf({ id: 'x', date: '2026-10-20' }), null, 'no start: not on this grid')
})

test('moving an event keeps how long it lasts', () => {
  const events = [event(), event({ id: 'b', start: '14:00', end: '15:00', title: '别的' })]
  const next = W.move(events, 'a', '2026-10-21', 14 * 60, '')
  const moved = next.find(e => e.id === 'a')
  assert.equal(moved.date, '2026-10-21', 'the column it was dropped in')
  assert.equal(moved.start, '14:00')
  assert.equal(moved.end, '15:30', 'the length followed it')
  assert.equal(moved.title, '周会', 'nothing else about it changed')
  // The others are where they were.
  assert.deepEqual(next.find(e => e.id === 'b'), events[1])
  assert.equal(next.length, 2)
})

test('an event of another account is not the one that moves', () => {
  // The same event in two accounts: moving one must not move the other.
  const events = [event({ account: 'acc-1' }), event({ account: 'acc-2' })]
  const next = W.move(events, 'a', '2026-10-21', 14 * 60, 'acc-1')
  assert.equal(next.find(e => e.account === 'acc-1').start, '14:00', 'the one that was asked for')
  assert.equal(next.find(e => e.account === 'acc-2').start, '09:00', 'the other stayed where it was')
  // And the grip likewise.
  const grown = W.resize(events, 'a', 11 * 60, 'acc-1')
  assert.equal(grown.find(e => e.account === 'acc-1').end, '11:00')
  assert.equal(grown.find(e => e.account === 'acc-2').end, '10:30')
})

test('an event with no end gets a quarter of an hour when it is moved', () => {
  const next = W.move([event({ end: undefined })], 'a', '2026-10-20', 600)
  const moved = next[0]
  assert.equal(moved.start, '10:00')
  assert.equal(moved.end, '10:15')
})

test('an id is matched as text, so a number from the page finds its event', () => {
  const events = [event({ id: 42 })]
  const next = W.move(events, '42', '2026-10-22', 480)
  assert.equal(next[0].date, '2026-10-22')
  assert.equal(next[0].start, '08:00')
})

test('the grip changes the end, never the start', () => {
  const grown = W.resize([event()], 'a', 11 * 60)[0]
  assert.equal(grown.start, '09:00', 'the start did not move')
  assert.equal(grown.end, '11:00')
  const shrunk = W.resize([event()], 'a', 9 * 60 + 30)[0]
  assert.equal(shrunk.start, '09:00')
  assert.equal(shrunk.end, '09:30')
})

test('the grip may not pull the end above the start', () => {
  // Dragged past the top: it stops a quarter of an hour after the start
  // rather than becoming a negative length.
  const over = W.resize([event()], 'a', 7 * 60)[0]
  assert.equal(over.start, '09:00')
  assert.equal(over.end, '09:15')
  // An event an hour long keeps the hour: the end goes back behind the start
  // as a block, never to the start itself.
  const hour = W.resize([event({ start: '09:00', end: '10:00' })], 'a', 9 * 60)[0]
  assert.equal(hour.end, '09:15')
})

test('a move or a resize with no event, or one with no time, is refused', () => {
  assert.equal(W.move([], 'a', '2026-10-21', 600), null, 'no such event')
  assert.equal(W.move([{ id: 'a', date: '2026-10-20', title: '全天' }], 'a', '2026-10-21', 600), null, 'no start: nothing to put on the grid')
  assert.equal(W.resize([{ id: 'a', date: '2026-10-20' }], 'a', 600), null)
  // A list that is not a list is an empty one, not a throw.
  assert.equal(W.move(undefined, 'a', '2026-10-21', 600), null)
})

test('the original list is never changed in place', () => {
  const events = [event()]
  const before = JSON.stringify(events)
  W.move(events, 'a', '2026-10-21', 600)
  W.resize(events, 'a', 1200)
  assert.equal(JSON.stringify(events), before, 'the page keeps the old list until calSave replaces it')
})