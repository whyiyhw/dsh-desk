// One-shot diagnostic for "connection lost" style issues: mint the auth cookie
// via /?token=..., then open the gateway websocket with it. Proves the server
// side (token -> cookie -> WS handshake) independently of the webview.
//
// Usage:
//   NODE_PATH=<dsh-checkout>/packages/api/gateway/node_modules \
//     node ws-probe.cjs <token> [base-url]
// This repo has no `ws` dependency — borrow it from a dsh checkout via NODE_PATH.
// <token> comes from a live `dsh web` instance; the (redacted) readiness line in
// %APPDATA%/dsh-desk/dsh-desk.log still carries the host:port for base-url.
const http = require('node:http')

const token = process.argv[2]
const base = process.argv[3] || 'http://127.0.0.1:3080'
if (!token) {
  console.log('usage: node ws-probe.cjs <token> [base-url]')
  process.exit(64)
}

http.get(`${base}/?token=${token}`, (res) => {
  const setCookie = res.headers['set-cookie']
  console.log('mint status:', res.statusCode, 'set-cookie:', setCookie ? 'yes' : 'no')
  if (!setCookie) { console.log('FAIL: no cookie minted'); process.exit(1) }
  const cookie = setCookie[0].split(';')[0]
  const WebSocket = require('ws')
  const wsUrl = base.replace(/^http/, 'ws') + '/api/remote.mux'
  const ws = new WebSocket(wsUrl, { headers: { cookie, origin: base, host: new URL(base).host } })
  const timer = setTimeout(() => { console.log('WS TIMEOUT (no open/close in 5s)'); process.exit(2) }, 5000)
  ws.on('open', () => { clearTimeout(timer); console.log('WS OPEN: server accepted the gateway websocket'); ws.close(); process.exit(0) })
  ws.on('error', (e) => { clearTimeout(timer); console.log('WS ERROR:', e.message); process.exit(1) })
  ws.on('unexpected-response', (req, r) => { clearTimeout(timer); console.log('WS REJECTED: HTTP', r.statusCode); process.exit(1) })
})
