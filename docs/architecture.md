# Architecture

How a request moves through the system, and where each guarantee is enforced.

## The shape of it

One process serves everything: server-rendered HTML, the hydration bundle, the
static assets, and every server function. There is no separate API service and no
reverse proxy in the container.

```
Browser
  │
  │  GET /signin                         POST /api/sign_in
  ▼                                      ▼
┌─────────────────────────────────────────────────────────┐
│ crates/server — Axum                                    │
│   middleware: panic capture, request id, tracing,       │
│               compression, body limit, security         │
│               headers, timeout                          │
│   routes:     /health/*  ·  SSR pages  ·  /api/*        │
└───────────────┬─────────────────────────────────────────┘
                │ provide_context(AppState)
                ▼
┌─────────────────────────────────────────────────────────┐
│ crates/app — Leptos (ssr)                               │
│   renders components · runs #[server] bodies            │
└───────────────┬─────────────────────────────────────────┘
                ▼
┌─────────────────────────────────────────────────────────┐
│ crates/domain — no Leptos anywhere                      │
│   auth (policy)  →  repo (SQL)  →  Postgres             │
└─────────────────────────────────────────────────────────┘
```

The same `crates/app` code compiles to wasm and hydrates the served HTML in the
browser. Under `hydrate`, the `#[server]` bodies are replaced by typed network
calls; the database code is not in the bundle at all, it simply is not compiled.

## Two paths through the same code

**A page load.** Axum matches an SSR route, `leptos_axum` provides the request
`Parts`, `ResponseOptions` and our `AppState` into the reactive context, and the
component tree renders to HTML. A `Resource` awaiting `current_user()` calls the
server function **in process** — no HTTP hop — so the shell knows whether the
visitor is signed in before a byte is sent. Serialized resource values travel in
the response so hydration does not refetch them.

**A form submission.** `<ActionForm>` posts to `/api/sign_in`. With JavaScript it
is intercepted and sent as a typed call; without it, the browser performs an
ordinary form post. Both arrive at the same handler. The server redirects with
`302` when the request accepts `text/html`, and with a header the client router
understands otherwise — which is why the app works either way.

## Layering rules

| Layer | May contain | Must not contain |
| --- | --- | --- |
| `core` | Types, parsing, serde | Anything platform-specific — it compiles to wasm |
| `domain/repo` | SQL | Policy decisions |
| `domain/auth` | Policy | SQL |
| `app` | UI, `#[server]` signatures | SQL, direct pool access |
| `server` | Routing, middleware | Business logic |

The `domain` crate exists because server functions must live in the UI crate
while needing database access; putting that access in `server` would make `app`
and `server` mutually dependent. Its side effect is the valuable part: **the
backend has no Leptos dependency**, so the UI can be replaced without touching
it. See [ADR 0003](adr/0003-five-crate-workspace-layout.md).

## Where each guarantee lives

**Validation happens once.** `Username::parse` and `Password::parse` in
`crates/core` are the only definition of the rules. The browser calls them for
live feedback; the server calls them before touching the database. Deserialization
routes through them too (`#[serde(try_from = "String")]`), so an invalid value
cannot enter the process even from a crafted payload.

**Authorization cannot be forgotten.** Every protected server function begins
with `ssr::require_user`, and ownership checks live inside the `DELETE` statement
rather than in a preceding query, so there is no window in which they could be
bypassed.

**Secrets cannot leak into logs.** `Password`, `SessionToken` and `Config` have
hand-written `Debug` implementations that redact their contents. A stray
`tracing::debug!` prints `SessionToken(<redacted>)`.

**Internal detail cannot leak to the browser.** `DomainError` carries the real
`sqlx::Error`; converting it to `ApiError` at the transport boundary collapses
every internal failure to `Internal` after logging the detail server-side,
correlated by request id.

**Sessions expire whether or not the sweep runs.** Every lookup filters on
`expires_at > now()`. The background task only stops the table growing.

## Request middleware, outermost first

| Layer | Why |
| --- | --- |
| `CatchPanicLayer` | A panic becomes a 500 for one request, not a dropped connection |
| `SetRequestIdLayer` / `PropagateRequestIdLayer` | Correlates logs; echoed so users can quote it |
| `TraceLayer` | Structured span per request with status and latency |
| `CompressionLayer` | brotli and gzip |
| `RequestBodyLimitLayer` | 256 KiB ceiling |
| Security headers | CSP, `nosniff`, `DENY`, referrer policy, permissions policy, HSTS in production |
| `TimeoutLayer` | 30 s. Innermost, so a timed-out response still gets the headers above — and because it needs an inner body implementing `Default`, which the wrapped bodies above do not provide |

## Known limitation: `unsafe-inline` in the CSP

`script-src` includes `'unsafe-inline'` because Leptos emits its hydration
bootstrap, and this app its theme-init script, as inline `<script>` elements.
`'wasm-unsafe-eval'` is also present; it permits WebAssembly compilation only,
not the much broader `'unsafe-eval'`.

Removing `'unsafe-inline'` means enabling leptos's `nonce` feature, calling
`use_nonce()` in the shell, and threading the nonce onto both scripts. It is the
one meaningful hardening step left in this stack. It is written down here rather
than left as a silent gap.

## Configuration that changes behaviour

`APP_ENV=production` switches on `Secure` cookies, the `__Host-` cookie prefix,
HSTS and JSON logging. `TRUSTED_PROXY_HOPS` must match the number of reverse
proxies actually in front of the process; the default `0` ignores
`X-Forwarded-For` entirely. See the [runbook](runbook.md) for the consequences of
getting either wrong.

## Testing strategy

| Level | Location | Covers |
| --- | --- | --- |
| Unit | alongside the code | Parsing, hashing, cookie construction, proxy resolution, rate limiting, error mapping |
| Integration | `crates/server/tests/` | The real router over real HTTP against a real database — one fresh database per test |
| Browser | `end2end/` | Auth flows, session management, accessibility, and a JavaScript-disabled suite |

The `no-javascript` Playwright project is the one that verifies the architecture
actually delivers what it claims: pages readable and forms usable before any wasm
executes.
