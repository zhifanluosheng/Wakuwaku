const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const { test } = require('node:test')

const { STRINGS } = require('../src/shared/i18n')
const Agents = require('../src/pet/agents')

const root = path.join(__dirname, '..')
const read = file => fs.readFileSync(path.join(root, file), 'utf8')

// A settings snapshot as settings.rs makes one: every agent on this
// computer and not connected, unless changed.
function snapshot(change = {}) {
  const block = (status, extra = {}) => ({ hooks: status, file: 'C:\\Users\\me\\x', fileShown: '~\\x', backup: false, seen: false, ...extra })
  return {
    connection: 'none',
    hooks: 'missing',
    claudeSeen: false,
    claudeHere: true,
    settingsFile: 'C:\\Users\\me\\.claude\\settings.json',
    settingsShown: '~\\.claude\\settings.json',
    settingsBackup: false,
    codex: block('missing', { version: '0.150.0', async: true }),
    agy: block('missing'),
    opencode: { plugin: 'missing', file: 'C:\\Users\\me\\.config\\opencode\\plugins\\wakuwaku.js', fileShown: '~\\.config\\opencode\\plugins\\wakuwaku.js', seen: false },
    omp: block('missing', { more: 0 }),
    zcode: block('missing'),
    trae: block('missing'),
    workbuddy: block('missing'),
    dsh: block('missing', { listShown: 'D:\\wakuwaku-data\\dsh-hooks.json' }),
    mailAgents: { claude: true, codex: true, agy: false },
    agentVersions: { claude: '2.1.296', zcode: '3.15.3', dsh: '0.2.0-rc.2', opencode: null, omp: null },
    sessions: [],
    ...change,
  }
}
const absent = { codex: { hooks: 'absent' }, agy: { hooks: 'absent' }, opencode: { plugin: 'absent' }, omp: { hooks: 'absent' }, zcode: { hooks: 'absent' }, trae: { hooks: 'absent' }, workbuddy: { hooks: 'absent' }, dsh: { hooks: 'absent' } }

test('each agent in the table is whole, and its words are there in both languages', () => {
  const bridge = read('src/pet/bridge.js')
  const sites = read('src-tauri/src/settings.rs')
  const keys = new Set()
  assert.equal(Agents.TABLE.length, 9)
  for (const a of Agents.TABLE) {
    assert.match(a.id, /^[a-z]+$/)
    assert.ok(a.name, a.id)
    assert.ok(['hooks', 'plugin', 'extension', 'bridge'].includes(a.via), `${a.id}: via ${a.via}`)
    // Its call is the bridge's, and its site one settings.rs may open.
    assert.match(bridge, new RegExp(`\\n\\s+${a.call}: action => invoke\\('settings_`), `${a.id}: bridge call ${a.call}`)
    assert.equal(a.site, `agent:${a.id}`)
    assert.match(sites, new RegExp(`\\("${a.id}", "https://`), `${a.id}: its site in AGENT_SITES`)
    assert.deepEqual(Object.keys(a.caps), Agents.CAPS, a.id)
    for (const [cap, [level, key]] of Object.entries(a.caps)) {
      assert.ok(['yes', 'part', 'no'].includes(level), `${a.id}.${cap}: ${level}`)
      keys.add(key)
    }
    if (a.caps.mail[0] !== 'no') keys.add(`ag.cap.mail.missing.${a.id}`)
    for (const k of ['what', 'note', 'how', 'st.set']) keys.add(`ag.${k}.${a.id}`)
    keys.add(`ag.via.${a.via}`)
    for (const k of Object.values(a.words || {})) keys.add(k)
    if (a.step) keys.add(a.step)
    if (a.stepNote) keys.add(a.stepNote)
    assert.ok(a.good.every(s => a.in.includes(s)), `${a.id}: good is part of in`)
  }
  for (const cap of Agents.CAPS) keys.add(`ag.cap.${cap}`)
  for (const id of ['claude', 'codex', 'agy']) keys.add(`mail.agentMissing.${id}`)
  for (const k of keys) {
    assert.ok(k in STRINGS.zh, `missing in zh: ${k}`)
    assert.ok(k in STRINGS.en, `missing in en: ${k}`)
  }
})

test('the table has every agent the server hears, and no other', () => {
  // server.rs: each agent's events by its ?agent= name, Claude Code's by none.
  const server = read('src-tauri/src/server.rs')
  const heard = [...server.matchAll(/Some\("(\w+)"\) => &sh\.(\w+)_seen/g)].map(m => m[1])
  assert.ok(heard.length >= 6, 'the scan is broken')
  assert.deepEqual(['claude', ...heard].sort(), Agents.TABLE.map(a => a.id).sort())
})

test('what she can do is what the code does', () => {
  const lit = cap => Agents.TABLE.filter(a => a.caps[cap][0] !== 'no').map(a => a.id).sort()
  // Approvals on the island: Claude Code's, and those panel.js names as
  // the other agents a prompt may come from.
  const panel = read('src/pet/panel.js')
  const from = Object.keys(eval(`(${/const FROM = (\{[^}]*\})/.exec(panel)[1]})`))
  assert.deepEqual(lit('approve'), ['claude', ...from].sort())
  // omp's are its plan reviews alone.
  assert.equal(Agents.byId('omp').caps.approve[0], 'part')
  // A question answered on the island: Claude Code's AskUserQuestion alone.
  assert.deepEqual(lit('choose'), ['claude'])
  // Mail: those mail/agent.rs runs.
  const mail = /const NAMES: \[&str; \d\] = \[([^\]]*)\]/.exec(read('src-tauri/src/mail/agent.rs'))[1]
  assert.deepEqual(lit('mail'), [...mail.matchAll(/"(\w+)"/g)].map(m => m[1]).sort())
  // Today counts every agent; tokens only those tokens.rs reads.
  assert.deepEqual(lit('stats'), Agents.TABLE.map(a => a.id).sort())
  // (tokens.rs keeps a count for each agent it reads, by its id.)
  const fields = /pub struct Tokens \{([\s\S]*?)\n\}/.exec(read('src-tauri/src/tokens.rs'))[1]
  const counted = new Set([...fields.matchAll(/\n\s+(\w+): /g)].map(m => m[1]))
  const withTokens = Agents.TABLE.filter(a => ['ag.cap.stats.tokens', 'ag.cap.stats.agy'].includes(a.caps.stats[1])).map(a => a.id)
  assert.deepEqual(withTokens, Agents.TABLE.filter(a => counted.has(a.id)).map(a => a.id))
  assert.deepEqual(withTokens, ['claude', 'codex', 'agy'])
})

test('found here or not', () => {
  const snap = snapshot()
  assert.deepEqual(Agents.here(snap), ['claude', 'codex', 'agy', 'opencode', 'omp', 'zcode', 'trae', 'workbuddy', 'dsh'])
  assert.deepEqual(Agents.elsewhere(snap), [])
  const none = snapshot({ claudeHere: false, ...absent })
  assert.deepEqual(Agents.here(none), [])
  assert.deepEqual(Agents.elsewhere(none), ['claude', 'codex', 'agy', 'opencode', 'omp', 'zcode', 'trae', 'workbuddy', 'dsh'])
  // Claude Code's folder gone, its plugin still on: here.
  assert.deepEqual(Agents.here(snapshot({ claudeHere: false, connection: 'plugin', ...absent })), ['claude'])
  // Only OpenCode and ZCode.
  const two = snapshot({ claudeHere: false, ...absent, opencode: { plugin: 'missing' }, zcode: { hooks: 'ok', seen: true } })
  assert.deepEqual(Agents.here(two), ['opencode', 'zcode'])
})

test('how each stands, and its one button', () => {
  const state = (change, id, known) => Agents.stateOf(snapshot(change), id, known)
  const action = (change, id, known) => Agents.actionOf(snapshot(change), id, known)
  // Not connected: Connect.
  assert.equal(state({}, 'codex'), 'none')
  assert.deepEqual(action({}, 'codex'), { act: 'install', label: 'ag.do.install' })
  // Written, waiting for its first turn: nothing to do, no button; an event
  // seen: connected, no button either (Reinstall and Remove are in its details).
  for (const id of ['agy', 'omp', 'zcode', 'dsh']) {
    assert.equal(state({ [id]: { hooks: 'ok', seen: false } }, id), 'set', id)
    assert.equal(action({ [id]: { hooks: 'ok', seen: false } }, id), null, id)
    assert.equal(state({ [id]: { hooks: 'ok', seen: true } }, id), 'on', id)
    assert.equal(action({ [id]: { hooks: 'ok', seen: true } }, id), null, id)
  }
  assert.equal(state({ opencode: { plugin: 'ok', seen: false } }, 'opencode'), 'set')
  // Codex written and not trusted yet: a step of the person's, its button opens the row.
  assert.equal(state({ codex: { hooks: 'ok', seen: false } }, 'codex'), 'step')
  assert.deepEqual(action({ codex: { hooks: 'ok', seen: false } }, 'codex'), { act: 'open', label: 'ag.do.trust' })
  assert.equal(state({ codex: { hooks: 'ok', seen: true } }, 'codex'), 'on')
  assert.equal(action({ codex: { hooks: 'ok', seen: true } }, 'codex'), null)
  assert.deepEqual(Agents.TABLE.filter(a => a.step).map(a => a.id), ['codex'])
  // Its standing note: while the trust shows in amber above it, only its
  // second half (the trust not said twice); once trusted, all of it.
  assert.equal(Agents.noteOf('codex', 'step'), 'ag.note.codexWake')
  for (const state of ['on', 'set', 'repair', 'none']) assert.equal(Agents.noteOf('codex', state), 'ag.note.codex', state)
  assert.equal(Agents.noteOf('dsh', 'set'), 'ag.note.dsh')
  for (const lang of ['zh', 'en']) {
    assert.ok(STRINGS[lang]['ag.note.codex'].endsWith(STRINGS[lang]['ag.note.codexWake']), lang)
    assert.ok(STRINGS[lang]['ag.note.codexWake'].length < STRINGS[lang]['ag.note.codex'].length / 2, lang)
  }
  // Ours there but to repair (moved, older, part missing, turned off): Repair.
  for (const [id, status] of [['codex', 'stale'], ['codex', 'upgradable'], ['codex', 'partial'], ['agy', 'stale'], ['agy', 'off'], ['omp', 'partial'], ['zcode', 'off'], ['zcode', 'partial'], ['dsh', 'stale']]) {
    assert.equal(state({ [id]: { hooks: status } }, id), 'repair', `${id} ${status}`)
    assert.deepEqual(action({ [id]: { hooks: status } }, id), { act: 'install', label: 'settings.repair' }, `${id} ${status}`)
  }
  assert.equal(state({ opencode: { plugin: 'stale' } }, 'opencode'), 'repair')
  // Its file unreadable: no button; written and failed: Retry; being written: none.
  assert.equal(state({ zcode: { hooks: 'unreadable' } }, 'zcode'), 'error')
  assert.equal(action({ zcode: { hooks: 'unreadable' } }, 'zcode'), null)
  const failed = { failed: { dsh: { error: 'locked', kind: 'locked' } } }
  assert.equal(state({}, 'dsh', failed), 'failed')
  assert.deepEqual(action({}, 'dsh', failed), { act: 'install', label: 'wc.toolRetry' })
  assert.equal(state({}, 'omp', { busy: 'omp' }), 'busy')
  assert.equal(action({}, 'omp', { busy: 'omp' }), null)
  // Connected since, a failure before does not count.
  assert.equal(state({ dsh: { hooks: 'ok', seen: true } }, 'dsh', failed), 'on')
})

test("Claude Code's two ways", () => {
  const state = change => Agents.stateOf(snapshot(change), 'claude')
  assert.equal(state({}), 'none')
  // The plugin: connected (no event is needed to know).
  assert.equal(state({ connection: 'plugin' }), 'on')
  // settings.json's hooks: written, then seen; nothing that starts her is fine too.
  assert.equal(state({ connection: 'hooks', hooks: 'ok' }), 'set')
  assert.equal(Agents.actionOf(snapshot({ connection: 'plugin' }), 'claude'), null)
  assert.equal(state({ connection: 'hooks', hooks: 'ok', claudeSeen: true }), 'on')
  assert.equal(state({ connection: 'hooks', hooks: 'httpOnly', claudeSeen: true }), 'on')
  assert.equal(state({ connection: 'hooks', hooks: 'stale' }), 'repair')
  assert.equal(state({ hooks: 'unreadable' }), 'error')
  // Both at once: Repair takes settings.json's out, the plugin stays.
  assert.equal(state({ connection: 'both', hooks: 'ok' }), 'repair')
  assert.deepEqual(Agents.actionOf(snapshot({ connection: 'both', hooks: 'ok' }), 'claude'), { act: 'remove', label: 'settings.repair' })
  assert.equal(Agents.read(snapshot(), 'claude').shown, '~\\.claude\\settings.json')
  assert.equal(Agents.read(snapshot(), 'claude').version, '2.1.296')
})

test('the files, as each reads them', () => {
  const snap = snapshot({ omp: { hooks: 'ok', fileShown: '~\\.omp\\agent\\extensions\\wakuwaku.ts', more: 2 } })
  assert.equal(Agents.read(snap, 'omp').more, 2)
  assert.equal(Agents.read(snap, 'omp').shown, '~\\.omp\\agent\\extensions\\wakuwaku.ts')
  assert.equal(Agents.read(snap, 'dsh').list, 'D:\\wakuwaku-data\\dsh-hooks.json')
  assert.equal(Agents.read(snap, 'opencode').shown, '~\\.config\\opencode\\plugins\\wakuwaku.js')
  assert.equal(Agents.read(snap, 'codex').version, '0.150.0')
  assert.equal(Agents.read(snap, 'zcode').version, '3.15.3')
  assert.equal(Agents.read(snap, 'agy').version, '')
  // Its own words for a status that wants something, else the shared ones.
  assert.equal(Agents.wordOf('omp', 'stale'), 's.ompStale')
  assert.equal(Agents.wordOf('codex', 'stale'), 'settings.hooksStale')
  assert.equal(Agents.wordOf('zcode', 'off'), 's.zcodeOff')
  assert.equal(Agents.wordOf('claude', 'both'), 'home.connBoth')
})

test('Connect all, and General’s line', () => {
  const snap = snapshot({ connection: 'plugin', codex: { hooks: 'ok', seen: false }, agy: { hooks: 'stale' }, ...{ omp: { hooks: 'absent' } } })
  // Here and not connected: OpenCode, ZCode, Trae, DeepSeek Harness (agy is to repair, omp is not here).
  assert.deepEqual(Agents.toConnect(snap), ['opencode', 'zcode', 'trae', 'workbuddy', 'dsh'])
  // One being written is not offered again; one that failed is.
  assert.deepEqual(Agents.toConnect(snap, { busy: 'zcode', failed: { dsh: { kind: 'locked' } } }), ['opencode', 'trae', 'workbuddy', 'dsh'])
  // Codex written, to be trusted: connected, and wanting the person (as agy's repair).
  const s = Agents.summary(snap)
  assert.deepEqual(s.on, ['claude'])
  assert.deepEqual(s.set, [])
  assert.deepEqual(s.connected, ['claude', 'codex'])
  assert.deepEqual(s.wants, ['codex', 'agy'])
  assert.deepEqual(s.none, ['opencode', 'zcode', 'trae', 'workbuddy', 'dsh'])
  // A file unreadable and a write that failed want the person too.
  const more = Agents.summary(snapshot({ zcode: { hooks: 'unreadable' }, omp: { hooks: 'ok' } }), { failed: { dsh: { kind: 'locked' } } })
  assert.deepEqual(more.wants, ['zcode', 'dsh'])
  assert.deepEqual(more.set, ['omp'])
  // Nothing wants the person: no tab dot.
  assert.deepEqual(Agents.summary(snapshot({ connection: 'plugin', codex: { hooks: 'ok', seen: true }, zcode: { hooks: 'ok' } })).wants, [])
})

test('the dots: four colours, by what the person has to do', () => {
  // Green connected, blue written and waiting, amber the person must act, grey not connected.
  assert.deepEqual(Agents.MOOD, { on: 'done', set: 'working', busy: 'working', step: 'waiting', repair: 'waiting', error: 'waiting', failed: 'waiting', none: 'idle' })
  assert.deepEqual([...new Set(Object.values(Agents.MOOD))].sort(), ['done', 'idle', 'waiting', 'working'])
  // Every state that wants the person is amber, and only those.
  for (const [state, mood] of Object.entries(Agents.MOOD)) assert.equal(mood === 'waiting', Agents.WANTS.includes(state), state)
})

test('mail: lit where its command is found, and the Mail page’s list', () => {
  const caps = (snap, id) => Object.fromEntries(Agents.capsOf(snap, id).map(([cap, level, key]) => [cap, [level, key]]))
  assert.deepEqual(caps(snapshot(), 'claude').mail, ['yes', 'ag.cap.mail.claude'])
  assert.deepEqual(caps(snapshot(), 'agy').mail, ['no', 'ag.cap.mail.missing.agy'])
  assert.deepEqual(caps(snapshot({ mailAgents: { agy: true } }), 'agy').mail, ['yes', 'ag.cap.mail.agy'])
  assert.deepEqual(caps(snapshot(), 'dsh').mail, ['no', 'ag.cap.mail.no'])
  assert.deepEqual(caps(snapshot(), 'dsh').approve, ['no', 'ag.cap.approve.dsh'])
  // Claude Code and Codex always; Antigravity where it is (its app alone:
  // greyed), found, or picked already.
  const ids = (snap, current) => Agents.mailAgents(snap, current).map(a => `${a.id}${a.found ? '' : '?'}`)
  assert.deepEqual(ids(snapshot(), 'claude'), ['claude', 'codex', 'agy?'])
  assert.deepEqual(ids(snapshot({ agy: { hooks: 'absent' } }), 'claude'), ['claude', 'codex'])
  assert.deepEqual(ids(snapshot({ agy: { hooks: 'absent' } }), 'agy'), ['claude', 'codex', 'agy?'])
  assert.deepEqual(ids(snapshot({ agy: { hooks: 'absent' }, mailAgents: { agy: true } }), 'claude'), ['claude?', 'codex?', 'agy'])
  assert.deepEqual(Agents.mailAgents(snapshot(), 'claude')[0], { id: 'claude', name: 'Claude Code', short: 'Claude', found: true })
})

test('sessions in sight, by agent', () => {
  const snap = snapshot({ sessions: [{ id: 'a' }, { id: 'b', agent: 'codex' }, { id: 'c', agent: 'codex' }, { id: 'd', agent: 'dsh' }] })
  assert.equal(Agents.sessionsOf(snap, 'claude'), 1)
  assert.equal(Agents.sessionsOf(snap, 'codex'), 2)
  assert.equal(Agents.sessionsOf(snap, 'omp'), 0)
})
