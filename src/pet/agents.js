// The agents she follows, in one table: each one's name, how she connects to
// it, where that is written, what she can do for it (its capabilities) and
// what to know about it; and from the settings' snapshot, how each stands.
// The Agent page draws from it, the welcome on General, General's status
// line and the Mail page's picker too, so a new agent is a row here (and its
// words in i18n.js) besides its connector (connection.rs, its events_*.rs,
// its snapshot block in settings.rs and its call in bridge.js). Loaded by
// the page before settings.js, and required by the tests.
//
// What she can do, five things, each yes, part or no, with the words that
// say so (a tooltip; the row's details list them), as the code has it:
//   status   follows its sessions (events.rs and each events_*.rs)
//   approve  answers its approvals on the island (asks.rs, panel.js): which
//            buttons; a PermissionRequest held for her
//   choose   answers a question with choices on the island (asks.rs
//            AskUserQuestion: Claude Code's alone; the others' questions
//            only make her wait, "detail.asksQuestion")
//   stats    counted in Today (main.rs count_today: every agent's turns and
//            time), and its tokens (tokens.rs: Claude Code, Codex, Antigravity)
//   mail     can read a letter for the Mail page (mail/agent.rs: claude,
//            codex, agy), when its command is found (snapshot mailAgents)
;(function (root) {
  const CAPS = ['status', 'approve', 'choose', 'stats', 'mail']
  // Windows' own glyphs for them (Segoe Fluent Icons): View, Accept,
  // BulletedList, AreaChart, Mail.
  const CAP_GLYPH = { status: '\uE890', approve: '\uE8FB', choose: '\uE8FD', stats: '\uE9D2', mail: '\uE715' }

  // Each one's own block of the snapshot (settings.rs): what of ours is
  // there (its status: absent when it is not on this computer), its file
  // and the same as read (~ for the home folder), whether a copy was kept,
  // whether an event came since she started; its version (Codex's in its
  // block, the others' where a file says it: agentVersions, connection.rs
  // versions). Claude Code's is spread over the snapshot, as it was first.
  const block = key => snap => {
    const b = snap[key] || {}
    return { status: b[key === 'opencode' ? 'plugin' : 'hooks'], file: b.file || '', shown: b.fileShown || b.file || '', backup: !!b.backup, seen: !!b.seen, version: b.version || (snap.agentVersions || {})[key] || '' }
  }

  // The table, in the order they are shown.
  //   name    as the island and the pages name it
  //   via     how she connects: hooks, plugin, extension, bridge
  //   call    the bridge's call that writes or removes it (bridge.js settings)
  //   site    where to get it (settings.rs settings_open_site, "agent:<id>")
  //   read    its part of the snapshot
  //   in      the statuses that mean ours is there (connected, or to repair)
  //   good    the ones of those that need nothing
  //   words   its own words for a status that wants something (wordOf)
  //   step    written, a step of the person's left before its first event
  //           (Codex: trusting the hooks in its /hooks): its button's words
  //   caps    [level, the words' key] for each of CAPS
  //   mail    its name on the Mail page (agent.rs Agent::name), if it reads mail
  const TABLE = [
    {
      id: 'claude',
      name: 'Claude Code',
      via: 'hooks',
      call: 'hooks',
      site: 'agent:claude',
      read: snap => ({
        // Through the plugin, settings.json's hooks, both, or neither.
        status: snap.connection === 'plugin' ? 'plugin' : snap.connection === 'both' ? 'both' : snap.hooks,
        file: snap.settingsFile || '',
        shown: snap.settingsShown || snap.settingsFile || '',
        backup: !!snap.settingsBackup,
        seen: !!snap.claudeSeen || snap.connection === 'plugin',
        here: snap.claudeHere !== false || (!!snap.connection && snap.connection !== 'none'),
        version: (snap.agentVersions || {}).claude || '',
      }),
      in: ['plugin', 'both', 'ok', 'httpOnly', 'stale', 'partial'],
      good: ['plugin', 'ok', 'httpOnly'],
      words: { both: 'home.connBoth', unreadable: 's.hooksUnreadable' },
      caps: {
        status: ['yes', 'ag.cap.status.claude'],
        approve: ['yes', 'ag.cap.approve.claude'],
        choose: ['yes', 'ag.cap.choose.claude'],
        stats: ['yes', 'ag.cap.stats.tokens'],
        mail: ['yes', 'ag.cap.mail.claude'],
      },
      mail: 'Claude',
    },
    {
      id: 'codex',
      name: 'Codex',
      via: 'hooks',
      call: 'codexHooks',
      site: 'agent:codex',
      read: block('codex'),
      in: ['ok', 'stale', 'upgradable', 'partial'],
      good: ['ok'],
      words: { upgradable: 's.codexUpgradable', unreadable: 's.codexUnreadable' },
      step: 'ag.do.trust',
      // Its standing note while the step's own words show above it: the
      // rest of it, not the trust twice.
      stepNote: 'ag.note.codexWake',
      caps: {
        status: ['yes', 'ag.cap.status.codex'],
        approve: ['yes', 'ag.cap.approve.codex'],
        choose: ['no', 'ag.cap.choose.codex'],
        stats: ['yes', 'ag.cap.stats.tokens'],
        mail: ['yes', 'ag.cap.mail.codex'],
      },
      mail: 'Codex',
    },
    {
      id: 'agy',
      name: 'Antigravity',
      via: 'hooks',
      call: 'agyHooks',
      site: 'agent:agy',
      read: block('agy'),
      in: ['ok', 'stale'],
      good: ['ok'],
      words: { off: 's.agyOff', unreadable: 's.agyUnreadable' },
      caps: {
        status: ['yes', 'ag.cap.status.agy'],
        approve: ['no', 'ag.cap.approve.agy'],
        choose: ['no', 'ag.cap.choose.agy'],
        stats: ['yes', 'ag.cap.stats.agy'],
        mail: ['yes', 'ag.cap.mail.agy'],
      },
      mail: 'Antigravity',
    },
    {
      id: 'opencode',
      name: 'OpenCode',
      via: 'plugin',
      call: 'opencode',
      site: 'agent:opencode',
      read: block('opencode'),
      in: ['ok', 'stale'],
      good: ['ok'],
      words: { stale: 's.opencodeStale', unreadable: 's.opencodeUnreadable' },
      caps: {
        status: ['yes', 'ag.cap.status.opencode'],
        approve: ['yes', 'ag.cap.approve.opencode'],
        choose: ['no', 'ag.cap.choose.opencode'],
        stats: ['yes', 'ag.cap.stats.turns'],
        mail: ['no', 'ag.cap.mail.no'],
      },
    },
    {
      id: 'omp',
      name: 'omp',
      // Its project's name, beside its command's.
      aka: 'oh-my-pi',
      via: 'extension',
      call: 'ompHooks',
      site: 'agent:omp',
      // Its extension in each profile's agent folder too: the first one's
      // path, and how many more.
      read: snap => ({ ...block('omp')(snap), more: snap.omp?.more || 0 }),
      in: ['ok', 'stale', 'partial'],
      good: ['ok'],
      words: { stale: 's.ompStale', partial: 's.ompPartial', unreadable: 's.ompUnreadable' },
      caps: {
        status: ['yes', 'ag.cap.status.omp'],
        approve: ['part', 'ag.cap.approve.omp'],
        choose: ['no', 'ag.cap.choose.omp'],
        stats: ['yes', 'ag.cap.stats.turns'],
        mail: ['no', 'ag.cap.mail.no'],
      },
    },
    {
      id: 'zcode',
      name: 'ZCode',
      via: 'hooks',
      call: 'zcodeHooks',
      site: 'agent:zcode',
      read: block('zcode'),
      in: ['ok', 'stale', 'partial'],
      good: ['ok'],
      words: { off: 's.zcodeOff', unreadable: 's.zcodeUnreadable' },
      caps: {
        status: ['yes', 'ag.cap.status.zcode'],
        approve: ['yes', 'ag.cap.approve.zcode'],
        choose: ['no', 'ag.cap.choose.zcode'],
        stats: ['yes', 'ag.cap.stats.turns'],
        mail: ['no', 'ag.cap.mail.no'],
      },
    },
    {
      id: 'trae',
      name: 'Trae CN',
      via: 'hooks',
      call: 'traeHooks',
      site: 'agent:trae',
      read: block('trae'),
      in: ['ok', 'stale', 'partial'],
      good: ['ok'],
      words: { unreadable: 's.traeUnreadable' },
      caps: {
        status: ['yes', 'ag.cap.status.trae'],
        approve: ['no', 'ag.cap.approve.trae'],
        choose: ['no', 'ag.cap.choose.trae'],
        stats: ['yes', 'ag.cap.stats.turns'],
        mail: ['no', 'ag.cap.mail.no'],
      },
    },
    {
      id: 'workbuddy',
      name: 'WorkBuddy',
      via: 'hooks',
      call: 'workbuddyHooks',
      site: 'agent:workbuddy',
      read: block('workbuddy'),
      in: ['ok', 'stale', 'partial'],
      good: ['ok'],
      words: { unreadable: 's.workbuddyUnreadable' },
      caps: {
        status: ['yes', 'ag.cap.status.workbuddy'],
        approve: ['no', 'ag.cap.approve.workbuddy'],
        choose: ['no', 'ag.cap.choose.workbuddy'],
        stats: ['yes', 'ag.cap.stats.turns'],
        mail: ['no', 'ag.cap.mail.no'],
      },
    },
    {
      id: 'dsh',
      name: 'DeepSeek Harness',
      via: 'bridge',
      call: 'dshHooks',
      site: 'agent:dsh',
      // The bridge's hook list, in her data folder, besides the patch file.
      read: snap => ({ ...block('dsh')(snap), list: snap.dsh?.listShown || '' }),
      in: ['ok', 'stale'],
      good: ['ok'],
      words: { unreadable: 's.dshUnreadable' },
      caps: {
        status: ['yes', 'ag.cap.status.dsh'],
        approve: ['no', 'ag.cap.approve.dsh'],
        choose: ['no', 'ag.cap.choose.dsh'],
        stats: ['yes', 'ag.cap.stats.dsh'],
        mail: ['no', 'ag.cap.mail.no'],
      },
    },
  ]

  const byId = id => TABLE.find(a => a.id === id) || null

  // The words for a status that wants something (to repair, unreadable),
  // its own where it has them (words), else these.
  const WORDS = { stale: 'settings.hooksStale', partial: 'settings.hooksPartial', unreadable: 'ag.st.unreadable', off: 'ag.st.off' }
  const wordOf = (id, status) => byId(id)?.words?.[status] || WORDS[status] || 'settings.hooksStale'

  // One's part of the snapshot, as each reads it; here unless its status
  // says it is absent (no folder of its own on this computer).
  function read(snap, id) {
    const a = byId(id)
    if (!a || !snap) return null
    const r = a.read(snap)
    return { ...r, here: r.here ?? (r.status !== undefined && r.status !== 'absent') }
  }

  // Ours is there: connected, or there to repair.
  const isIn = (snap, id) => {
    const r = read(snap, id)
    return !!r && byId(id).in.includes(r.status)
  }

  // How one stands, for its dot and its button:
  //   busy     being written now
  //   on       connected, an event seen (a session came)
  //   set      written, no event yet: nothing for the person to do but use it
  //   step     written, no event yet, and a step of the person's first
  //            (Codex: trusting the hooks; the table's step)
  //   repair   ours there but not as this copy would write it (moved, an
  //            older version, a part missing, turned off, both of Claude
  //            Code's ways at once)
  //   error    its file can't be read
  //   failed   the last write failed (why: the page keeps it)
  //   none     not connected
  // The page passes what it knows: { busy: id being written, failed: { id: why } }.
  function stateOf(snap, id, { busy = '', failed = {} } = {}) {
    if (busy === id) return 'busy'
    const a = byId(id)
    const r = read(snap, id)
    if (!a || !r) return 'none'
    if (a.in.includes(r.status)) return !a.good.includes(r.status) ? 'repair' : r.seen ? 'on' : a.step ? 'step' : 'set'
    if (r.status === 'off') return 'repair'
    if (failed[id]) return 'failed'
    if (r.status === 'unreadable') return 'error'
    return 'none'
  }

  // The states that want the person (a repair, a step, a file to put right,
  // a write that failed): amber, and the Agent tab's dot.
  const WANTS = ['step', 'repair', 'error', 'failed']

  // The mood whose colour a state's dot takes (the island's, settings.js
  // COLOR), four of them: green connected, blue written and waiting for
  // its first turn, amber the person must act, grey not connected. Colour
  // says how it stands, never which one it is.
  const MOOD = { on: 'done', set: 'working', busy: 'working', step: 'waiting', repair: 'waiting', error: 'waiting', failed: 'waiting', none: 'idle' }

  // Its row's button, only where the person has something to do:
  //   install   connect (or try again), or repair
  //   remove    Claude Code with both ways at once: settings.json's hooks
  //             taken out, the plugin kept (its Repair)
  //   open      a step to take (Codex's trust): the row opens to say how
  //   null      none: connected, written and waiting, being written, or its
  //             file unreadable (to be put right by hand: its details say so)
  // Reinstall and Remove are in the open row's details.
  function actionOf(snap, id, known) {
    const state = stateOf(snap, id, known)
    if (state === 'none' || state === 'failed') return { act: 'install', label: state === 'failed' ? 'wc.toolRetry' : 'ag.do.install' }
    if (state === 'repair') return id === 'claude' && read(snap, id).status === 'both' ? { act: 'remove', label: 'settings.repair' } : { act: 'install', label: 'settings.repair' }
    if (state === 'step') return { act: 'open', label: byId(id).step }
    return null
  }

  // The ones on this computer, and the others, in the table's order.
  const here = snap => TABLE.filter(a => read(snap, a.id)?.here).map(a => a.id)
  const elsewhere = snap => TABLE.filter(a => !read(snap, a.id)?.here).map(a => a.id)

  // Those on this computer and not connected, for Connect all.
  const toConnect = (snap, known) => here(snap).filter(id => ['none', 'failed'].includes(stateOf(snap, id, known)))

  // What she can do for one, each of CAPS: [cap, level, the words' key].
  // Mail where its command is found (agent.rs available): else no, and why.
  function capsOf(snap, id) {
    const a = byId(id)
    return CAPS.map(cap => {
      const [level, key] = a.caps[cap]
      if (cap === 'mail' && level !== 'no' && snap && !(snap.mailAgents || {})[id]) return [cap, 'no', `ag.cap.mail.missing.${id}`]
      return [cap, level, key]
    })
  }

  // General's status line and the Agent tab's dot: the ones on this
  // computer by how they stand. connected: written (Codex's still to be
  // trusted too); wants: those the person must act on (amber).
  function summary(snap, known) {
    const out = { here: here(snap), on: [], set: [], wants: [], none: [] }
    for (const id of out.here) {
      const state = stateOf(snap, id, known)
      if (state === 'on' || state === 'set') out[state].push(id)
      else if (WANTS.includes(state)) out.wants.push(id)
      else out.none.push(id)
    }
    out.connected = out.here.filter(id => out.on.includes(id) || out.set.includes(id) || stateOf(snap, id, known) === 'step')
    return out
  }

  // The Mail page's picker: those that read mail, each found (its command
  // here) or not, and why not. Antigravity only where it is, or picked
  // already (few have it; zhoupengjie/Wakuwaku#42): its app alone brings no
  // agy, so it is listed then, greyed.
  function mailAgents(snap, current) {
    const found = snap.mailAgents || {}
    return TABLE.filter(a => a.mail)
      .filter(a => a.id !== 'agy' || found.agy || current === 'agy' || read(snap, 'agy').here)
      .map(a => ({ id: a.id, name: a.name, short: a.mail, found: !!found[a.id] }))
  }

  // How many sessions of one are in sight now (state.rs: agent, none for Claude Code's).
  const sessionsOf = (snap, id) => (snap.sessions || []).filter(s => (s.agent || 'claude') === id).length

  // The standing note at the foot of one's details (ag.note.<id>); while
  // its step shows in amber at their top, the part that does not say it again.
  const noteOf = (id, state) => (state === 'step' && byId(id)?.stepNote) || `ag.note.${id}`

  const api = { CAPS, CAP_GLYPH, TABLE, MOOD, WANTS, byId, noteOf, wordOf, read, isIn, stateOf, actionOf, here, elsewhere, toConnect, capsOf, summary, mailAgents, sessionsOf }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api
  } else {
    root.Agents = api
  }
})(this)
