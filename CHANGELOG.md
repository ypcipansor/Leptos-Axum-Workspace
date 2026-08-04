# Changelog

Notable changes to this project. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Complete rewrite. The previous code was a client-rendered SPA and a separate
REST API that happened to share a workspace; this is one application.

Anyone tracking the old layout should treat this as a new starting point rather
than an upgrade — the crate structure, the database schema, and the client/server
contract all changed.

### Added

- **Server-side rendering with hydration.** `cargo-leptos`, `leptos_axum`,
  `leptos_router` and `leptos_meta`. Pages arrive as complete HTML and are usable
  before any wasm loads; a dedicated Playwright project verifies this with
  JavaScript disabled. See [ADR 0001](docs/adr/0001-server-rendered-leptos-with-axum.md).
- **Server functions** replacing the hand-written REST client. One definition
  produces the handler and the typed call, so the network boundary is checked at
  compile time.
- **Versioned migrations**, applied at startup under an advisory lock.
- **`crates/domain`**, a backend layer with no Leptos dependency, so the UI can be
  replaced without touching it. See [ADR 0003](docs/adr/0003-five-crate-workspace-layout.md).
- **Structured logging** with `tracing`, request-id correlation, and JSON output
  in production.
- **Separate liveness and readiness probes** at `/health/live` and
  `/health/ready`.
- **Graceful shutdown** on `SIGTERM` and `SIGINT`.
- **URL routing** with deep links, working browser history, and a real 404 page
  returning a 404 status.
- **Dark mode** applied before first paint, and a reusable component set that
  owns its own accessibility behaviour.
- **Accessibility tests** with axe-core across every page in both themes.
- **Supply-chain checks**: `cargo-deny`, CodeQL, dependency review, weekly
  scheduled scans.
- **Release pipeline** publishing a multi-architecture image to GHCR with an SBOM
  and build provenance.
- `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, an architecture
  overview, an operations runbook, and three ADRs.

### Changed

- **Password hashing** from bcrypt to **Argon2id**, run on the blocking pool.
  bcrypt silently truncates input at 72 bytes.
- **Session tokens** from a UUIDv4 stored in plaintext to 256 bits of CSPRNG
  output stored only as a SHA-256 digest.
- **Token transport** from a JSON body plus `localStorage` to an `HttpOnly`
  cookie with `SameSite=Lax`, and `Secure` plus the `__Host-` prefix in
  production.
- **Session schema** now references `users(id)` instead of the mutable
  `users(username)`, and carries `expires_at`, `last_seen_at`, `user_agent` and
  an `INET` address.
- **Queries** from the unchecked `sqlx::query()` with `row.get("column")` to the
  compile-time-verified `query_as!` family, with a committed offline cache.
- **Errors** from repeated inline `match` blocks returning a fixed string to a
  typed `DomainError`, converted to a safe `ApiError` at the boundary.
- **Configuration** from `env::var` calls scattered across modules — with a
  hard-coded database URL as a fallback — to a typed `Config` validated at boot.
- **Tailwind** from v3 fetched at build time via `npx tailwindcss@3.4.15` to v4
  driven by cargo-leptos, removing Node.js from the build entirely.
- **Container** from two images plus nginx to one, built with `cargo-chef` layer
  caching, running as a non-root user with a healthcheck.
- **CI** now lints every crate on both native and wasm targets, has concurrency
  cancellation and timeouts, and pins every action by commit SHA.
- **`AGENTS.md`** rules are now ones CI can enforce, replacing a requirement for
  before/after screenshots that neither CI nor an agent could produce.
- Documentation is in English throughout.

### Fixed

Security issues in the previous implementation:

- Session tokens were stored **in plaintext** as the primary key of the sessions
  table. A database dump was a set of working credentials.
- Tokens lived in `localStorage`, readable by any injected script.
- Sessions **never expired**. There was no expiry column and nothing removed a
  row.
- Sign-in and registration had **no rate limiting** of any kind.
- `X-Forwarded-For` was read from its leftmost, fully attacker-controlled entry,
  then replaced with `127.0.0.1` when absent — so every recorded address was
  either forged or fabricated.
- CORS allowed any header and any method.
- A missing `FRONTEND_ORIGIN` panicked at startup instead of failing with a
  message.
- Database credentials were hard-coded in source as a fallback.
- No security headers were set at all.
- No request body limit and no request timeout.
- The timing-attack mitigation used a hard-coded bcrypt hash that could not
  follow changes to the algorithm or its parameters.
- Registration performed a redundant existence check, leaving a race between the
  check and the insert.

Correctness and quality issues:

- `list_sessions` returned HTTP 500 with an empty array as the body, which a
  client could not distinguish from an empty result.
- The frontend crate was excluded from every CI check, so none of its Rust was
  ever linted.
- Data loading used `Effect` plus `spawn_local`, which renders nothing on the
  server.
- Validation rules existed twice, in the browser and in the handler, with nothing
  keeping them in step.
- `.expect()` in wasm code panicked in the browser.
- Timestamps crossed the network pre-formatted as strings, so they could not be
  rendered in the viewer's locale.
- Inputs sat outside any `<form>`, with Enter handled by a keydown listener, and
  status messages rendered into elements no screen reader announced.
- There was no `.dockerignore`, so every build uploaded `target/` as context.

[Unreleased]: https://github.com/analisaperlengkapan/leptos-axum-workspace/commits/main
