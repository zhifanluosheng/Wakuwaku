// A fake CalDAV server, for the calendar's own tests: it answers PROPFIND
// with a calendar (name and colour in Chinese) and REPORT with events whose
// titles are Chinese, so that what comes back can be read.
import http from 'node:http'

const NAME = '工作日历'
const COLOR = '#FF3B30'
const EVENTS = [
  { uid: 'u1', summary: '周会讨论', start: '20261020T090000', end: '20261020T100000' },
  { uid: 'u2', summary: '设计评审：第三版', start: '20261021T140000', end: '20261021T150000' },
]

const ics = e =>
  [
    'BEGIN:VCALENDAR',
    'VERSION:2.0',
    'PRODID:-//wakuwaku//test//EN',
    'BEGIN:VEVENT',
    `UID:${e.uid}`,
    `DTSTART:${e.start}`,
    `DTEND:${e.end}`,
    `SUMMARY:${e.summary}`,
    'END:VEVENT',
    'END:VCALENDAR',
  ].join('\r\n')

const propfind = `<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:a="http://apple.com/ns/ical/">
<d:response>
  <d:href>/caldav/1/work/</d:href>
  <d:propstat><d:prop>
    <d:displayname>${NAME}</d:displayname>
    <d:resourcetype><d:collection/><c:calendar/></d:resourcetype>
    <c:calendar-color>${COLOR}</c:calendar-color>
  </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
</d:response>
</d:multistatus>`

const report = `<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
${EVENTS.map(
  e => `<d:response>
  <d:href>/caldav/1/work/${e.uid}.ics</d:href>
  <d:propstat><d:prop>
    <d:getetag>"${e.uid}-1"</d:getetag>
    <c:calendar-data>${ics(e).replace(/&/g, '&amp;').replace(/</g, '&lt;')}</c:calendar-data>
  </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
</d:response>`,
).join('\n')}
</d:multistatus>`

const server = http.createServer((req, res) => {
  let body = ''
  req.on('data', c => (body += c))
  req.on('end', () => {
    const isPropfind = req.method === 'PROPFIND'
    const xml = isPropfind ? propfind : report
    res.writeHead(207, { 'Content-Type': 'application/xml; charset=utf-8' })
    res.end(xml)
  })
})
server.listen(14713, '127.0.0.1', () => console.log('fake caldav on 14713'))