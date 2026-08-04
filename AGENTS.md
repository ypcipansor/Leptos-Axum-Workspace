# Working in this repository

Guidance for anyone — human or agent — changing this codebase.

## The one rule

**`just ci` must pass.** It runs exactly what the pipeline runs, in the same
order: format check, clippy on both targets, tests, dependency audit, release
build. If it passes locally it passes in CI.

Every rule below is enforced by that command or by the compiler. Nothing here
depends on remembering to do something.

## Layering

Dependencies flow one way: `core ← domain ← app ← {frontend, server}`.

| Crate | Holds | Never holds |
| --- | --- | --- |
| `app-core` | Types, parsing, serde | Filesystem, clock, randomness — it compiles to wasm |
| `app-domain` | Config, SQL, auth policy | Any Leptos or routing code |
| `app` | Components, routes, `#[server]` signatures | SQL, direct pool access |
| `frontend` | The wasm entrypoint | Anything else |
| `server` | Router, middleware, health, shutdown | Business logic |

SQL belongs in `domain/repo` and nowhere else. Policy belongs in `domain/auth`
and nowhere else. See [`docs/adr/0003`](docs/adr/0003-five-crate-workspace-layout.md)
for why the crates are split this way.

## Requirements for a change

**A schema change needs a migration.** Never edit an applied migration file —
sqlx records a checksum and startup will fail. Add a new one:

```bash
just migrate-new add_widget_table
just migrate
just prepare        # refresh .sqlx, or CI fails on a stale cache
```

Both `.up.sql` and `.down.sql` must be filled in; a migration you cannot reverse
is one you cannot deploy confidently.

**A new query needs `just prepare`.** The `.sqlx` directory is committed so
`cargo check` works without a database. CI verifies it matches the migrations.

**A new server function needs a test.** Add it to `crates/server/tests/auth.rs`
or a sibling file. Integration tests run against the real router and a real
database, so a test that passes there reflects production behaviour.

**A new validation rule goes in `app-core`.** It then runs in the browser and on
the server automatically. Adding a check inside a handler creates the second copy
this design exists to eliminate.

**A new UI component owns its accessibility.** Real `<label for>`, `aria-invalid`
on invalid inputs, `aria-describedby` linking help and error text, a live region
for anything that should be announced. `just e2e` includes an axe-core scan that
will fail otherwise.

## Security expectations

These are not style preferences; each corresponds to a defect that was fixed.

- Secrets get a hand-written `Debug` that redacts. See `Password`,
  `SessionToken`, `Config`.
- Never log a token, a password, or a connection string.
- Internal errors convert to `ApiError::Internal` at the transport boundary.
  Detail goes to the log, correlated by request id — never to the browser.
- Authentication failures must be indistinguishable from one another. "No such
  user" and "wrong password" return the same response and spend the same CPU.
- Ownership checks belong inside the SQL statement, not in a preceding query.

## Conventions

**Comments explain why, not what.** `// increment the counter` above `i += 1` is
noise. A comment earning its place answers a question the reader would otherwise
have to research: why a value was chosen, what breaks without a line, which
alternative was rejected and why.

**Tests are named as claims.** `an_expired_session_no_longer_authenticates`, not
`test_session_3`. The name should tell a reader what broke when it fails.

**No `unwrap` or `expect` outside tests.** The workspace lints warn on them and
CI escalates warnings to errors. Return a typed error instead.

**English throughout** — code, comments, commit messages, documentation and UI.

## Commits and pull requests

Present tense, imperative, explaining the reason:

```
Reject X-Forwarded-For unless a proxy hop count is configured

The header is attacker-controlled, so trusting it unconditionally let a
client choose the address recorded against its own sessions.
```

Pull requests should say what changed and why, and flag anything a reviewer
should look at closely. If a change alters security behaviour, say so explicitly.

## What was removed from this file

An earlier version required a before/after screenshot for every UI change.
Neither CI nor an agent can produce or verify one, so it was advice that could
not be enforced and in practice was not followed. The accessibility and
no-JavaScript suites in `end2end/` check the properties those screenshots were
meant to protect, and they fail the build when something regresses.
