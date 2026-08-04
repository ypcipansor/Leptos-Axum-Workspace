# `server`

The Axum binary: routing, middleware, health probes, graceful shutdown.

## Thin by design

This crate wires things together. Business logic belongs in `app-domain`, UI in
`app`. If you are writing a decision here, it is in the wrong place.

It is split into a library and a binary so integration tests can build the exact
router the binary serves, rather than an approximation of it.

## Middleware ordering

The stack in `router.rs` is ordered deliberately; the comments there explain each
position. Two constraints are easy to break by accident:

- **`TimeoutLayer` must be innermost.** It synthesises a 408 response, which
  requires the inner body to implement `Default` — the wrapped bodies produced by
  the compression and body-limit layers do not. Moving it outward is a compile
  error whose message does not obviously say so.
- **Security headers must be outside the timeout**, or a timed-out response
  escapes without them.

## Startup order

`Config::from_env` → `telemetry::init` → pool → **migrate** → listener.

Migrations run before the listener opens, so an instance never accepts traffic
against a schema it does not understand. sqlx takes an advisory lock, so several
replicas starting at once is safe.

`main` returns `Result`. Nothing here panics on a configuration problem; the
error names the variable and the process exits non-zero.

## Health probes

Keep `/health/live` free of any dependency check. It answers "is this process
alive", and an orchestrator uses it to decide whether to **restart**. Adding a
database check there turns a brief connectivity blip into a restart loop.

`/health/ready` is where dependencies belong — it decides whether to **route**.

## Integration tests

`tests/common/mod.rs` gives each test its own database and its own server on an
ephemeral port. Tests are therefore independent of order and safe in parallel.

Assert on the serialized error code — `unauthenticated`, `rate_limited` — not on
the human-readable message, which is UI wording and may change.

To simulate an expired session, age `created_at` alongside `expires_at`. The
schema enforces `expires_at > created_at`, so a session cannot be made to look as
though it expired before it began — and a genuinely expired session is an old
one. `TestApp::expire_sessions` does this correctly.
