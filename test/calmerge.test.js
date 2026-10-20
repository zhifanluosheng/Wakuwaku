// The calendar page's merge of what the accounts hold: an event is told
// apart by the account it is in and its own id, so two accounts holding the
// same event do not hide each other, and one account losing an event does
// not take the other's away.
//
// The rules are settings.js's (calEventKey, calSameEvent and the merge in
// the sync handler); they are read here as the page has them.
const assert = require('node:assert/strict')
const { test } = require('node:test')

// What the page keeps: the events it has, the ones the sync brought back,
// and the ones the sync says are gone.
function merge(have, pulled, removed, activeId) {
  const key = e => `${e && e.account ? e.account : activeId}${e ? e.id : ''}`
  const map = new Map(have.map(e => [key(e), e]))
  for (const e of pulled) {
    map.set(key(e), e)
    // An event made before accounts had one has no account of its own: the
    // copy that came back says which account it is in, and takes the old
    // one's place rather than standing beside it.
    for (const [k, old] of map) {
      if (k !== key(e) && !old.account && String(old.id) === String(e.id)) map.delete(k)
    }
  }
  for (const gone of removed) {
    const k = `${gone.account || activeId}${gone.id}`
    map.delete(k)
    for (const [kk, old] of map) {
      if (!old.account && String(old.id) === String(gone.id)) map.delete(kk)
    }
  }
  return [...map.values()]
}

const ev = (id, account, over = {}) => ({ id, account, date: '2026-10-20', start: '09:00', end: '10:00', title: id, ...over })

test('the same event in two accounts is two events', () => {
  // iCloud and a shared calendar both hold it: the same UID, two places.
  const out = merge([ev('uid-1', 'acc-icloud')], [ev('uid-1', 'acc-other')], [], 'acc-icloud')
  assert.equal(out.length, 2, 'one did not hide the other')
  assert.deepEqual(out.map(e => e.account).sort(), ['acc-icloud', 'acc-other'])
})

test('one account losing an event does not take the other\'s away', () => {
  const have = [ev('uid-1', 'acc-icloud'), ev('uid-1', 'acc-other')]
  const out = merge(have, [], [{ account: 'acc-icloud', id: 'uid-1' }], 'acc-icloud')
  assert.equal(out.length, 1, 'only the one that was lost is gone')
  assert.equal(out[0].account, 'acc-other')
})

test('an event made before accounts had one is not counted twice', () => {
  // The local copy has no account; the one that came back says which
  // account it is in. They are the same event, so there is one.
  const out = merge([ev('uid-1', '')], [ev('uid-1', 'acc-icloud')], [], 'acc-icloud')
  assert.equal(out.length, 1)
  assert.equal(out[0].account, 'acc-icloud')
})

test('an event of another account is not taken for this one\'s', () => {
  const have = [ev('uid-1', 'acc-other', { title: '别的' })]
  const out = merge(have, [ev('uid-1', 'acc-icloud', { title: '这个' })], [], 'acc-icloud')
  assert.equal(out.length, 2)
  assert.equal(out.find(e => e.account === 'acc-other').title, '别的')
  assert.equal(out.find(e => e.account === 'acc-icloud').title, '这个')
})

test('an event with no account belongs to the current one', () => {
  const out = merge([ev('uid-1', '')], [], [], 'acc-icloud')
  assert.equal(out.length, 1)
  // And a removal said with no account takes that one away.
  const gone = merge(out, [], [{ id: 'uid-1' }], 'acc-icloud')
  assert.equal(gone.length, 0)
})

test('nothing is lost that was not said to be gone', () => {
  const have = [ev('a', 'acc-1'), ev('b', 'acc-1'), ev('c', 'acc-2')]
  const out = merge(have, [], [], 'acc-1')
  assert.equal(out.length, 3)
  assert.deepEqual(out.map(e => e.id).sort(), ['a', 'b', 'c'])
})