# `app-domain`

Server-side logic: configuration, migrations, repositories, authentication.

## No Leptos, ever

This crate must stay free of `leptos`, `leptos_axum`, `leptos_router` and
anything else UI-shaped. Two reasons:

1. It exists to break a dependency cycle — server functions live in the UI crate
   but need database access.
2. Keeping it framework-free is the concrete mitigation for depending on a
   lightly maintained framework. If the UI layer is ever replaced, this crate is
   untouched. That property is only real while the dependency list stays clean.

`http` is fine — it is framework-agnostic and lets the trusted-proxy logic be
unit tested without an HTTP server.

## Where things belong

| Module | Contains | Never contains |
| --- | --- | --- |
| `repo/` | Every SQL statement | Policy decisions |
| `auth/` | Policy: who may do what, and at what cost | SQL |
| `config` | Environment parsing, validated at boot | Runtime lookups |
| `db` | Pool and migrations | Queries |

Nothing above `repo` writes SQL. Nothing inside `repo` decides anything.

## Queries

Use the `query_as!` family, never the unchecked `sqlx::query()` with
`row.get("column")`. The macros verify column names, nullability and types
against the real schema at compile time.

Repository functions return structs of primitives, not the newtypes from
`app-core`. Conversion happens in the service layer, so one unparseable legacy
row cannot make an entire query fail to decode. When conversion does fail, that
is `DomainError::DataIntegrity` — a fault to investigate, not a validation
message to show a user.

After changing any query, run `just prepare`.

## Authentication invariants

Each of these corresponds to a defect that was fixed. Preserve them.

- **Tokens are never stored.** Only `SessionToken::hash()` reaches the database.
- **Expiry is checked in SQL.** Every lookup filters `expires_at > now()`, so a
  stale session is unusable whether or not the sweep has run.
- **Failures are indistinguishable.** An unknown username runs
  `password::verify_dummy` so it costs the same as a wrong password. Removing it
  makes usernames enumerable by response time.
- **Ownership is in the statement.** `delete_for_user` filters on `user_id`
  inside the `DELETE`; a row belonging to someone else reports "not found", never
  "forbidden".
- **Only failures count toward the rate limit.** A successful sign-in clears the
  budget, so ordinary multi-device use never approaches a lockout.

## Password hashing

Argon2id at the OWASP defaults, on the blocking pool. Never hash on an async
worker thread — it costs tens of milliseconds of CPU and would stall every other
future scheduled there.

Parameters are embedded in the stored PHC string, so raising them later does not
invalidate existing hashes.
