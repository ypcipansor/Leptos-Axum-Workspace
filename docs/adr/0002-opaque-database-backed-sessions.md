# 2. Opaque, database-backed session cookies

- **Status:** Accepted
- **Date:** 2026-08-04

## Context

The previous authentication scheme had four high-severity problems at once:

1. The session token was stored **in plaintext** in the database, as the primary
   key of the `sessions` table. A dump of that table was a set of working
   credentials.
2. The token was returned in a JSON body and kept in `localStorage`, readable by
   any script on the page.
3. Sessions **never expired**. There was no `expires_at` column and nothing ever
   removed a row.
4. There was no rate limiting on sign-in or registration.

Passwords used bcrypt 0.15, which silently truncates input at 72 bytes.

Six mechanisms were evaluated against six weighted criteria.

| Option | Score |
| --- | --- |
| **B1 — Opaque cookie, database-backed** | **4.75** |
| B2 — `tower-sessions` + `axum-login` | 3.85 |
| B6 — Delegate to an external OIDC provider | 3.65 |
| B4 — Short-lived JWT plus opaque refresh token | 3.00 |
| B3 — Stateless encrypted cookie | 2.85 |
| B5 — Bearer token in `localStorage`, repaired | 2.35 |

## Decision

Adopt **B1**. A sign-in mints 256 bits of CSPRNG output, returns it in an
`HttpOnly` cookie, and stores only its SHA-256 digest. Passwords use Argon2id.

Specifically:

- **Token**: 32 random bytes from the OS CSPRNG, base64url encoded. The previous
  scheme used a UUIDv4 — 122 bits with advertised structure.
- **Storage**: `token_hash BYTEA` with a unique index and a 32-byte check
  constraint. SHA-256 rather than a password hash, because the input already has
  full entropy and this lookup is on every authenticated request.
- **Cookie**: `HttpOnly`, `SameSite=Lax`, `Path=/`, `Max-Age` matching the
  session lifetime. In production, `Secure` and the `__Host-` prefix, which the
  browser enforces and which makes the cookie unsettable by a sibling subdomain.
- **Expiry**: `expires_at NOT NULL`, checked in the `WHERE` clause of every
  lookup, so an expired session is unusable before any sweep runs. The window
  slides forward on use, throttled to at most one write per 5% of the lifetime.
- **Passwords**: Argon2id at OWASP default parameters, hashed on the blocking
  pool. Parameters are embedded in the PHC string, so they can be raised later
  without invalidating existing hashes.
- **Rate limiting**: keyed on address *and* username, so a distributed attempt
  against one account is throttled while a shared office address is not.

## Consequences

Revocation is immediate: deleting the row ends the session on the next request.
A leaked database dump yields no usable credential. Cross-device session listing
and revocation — already this template's headline feature — are directly
supported, because the store is queryable.

The cost is one indexed lookup per authenticated request, and roughly 250 lines
of code we own.

## Why not the alternatives

*B3 (stateless encrypted cookie)* cannot revoke a session before it expires,
which removes the feature the template exists to demonstrate. Effectively
self-disqualifying.

*B4 (JWT)* leaves an access token valid for its full lifetime after sign-out. For
a single-binary application it adds key rotation and refresh-reuse detection
while buying nothing.

*B5 (bearer token in `localStorage`)* is structurally exposed to XSS and cannot
be read by the server during rendering, so it is incompatible with SSR.

*B2 (`tower-sessions` + `axum-login`)* is well designed and was the closest call.
It was rejected on maintenance risk rather than design: `axum-login` 0.18 last
shipped in July 2025 and `tower-sessions-sqlx-store` in January 2025. Taking on a
second maintenance risk, on top of the framework's, to save ~250 lines of an
extremely standard pattern is a poor trade for something meant to last. Its
schema also fixes the session table shape, which conflicts with the
`user_agent` / `ip_address` / `last_seen_at` columns the session list needs.

*B6 (external OIDC)* has the smallest attack surface and is the natural upgrade
once an organisation is behind the application. As a starting point it would
require every user of this template to stand up an identity provider before
`just dev` works. Note that B1 is a prerequisite for B6, not a competitor: adding
OIDC later changes how identity is *established*, while the session layer stays.

## Related fixes

- Client addresses come from the TCP peer unless `TRUSTED_PROXY_HOPS` declares
  how many proxies are in front. The previous code read the leftmost, fully
  attacker-controlled entry of `X-Forwarded-For` and substituted `127.0.0.1`
  when it was absent, so every recorded address was forged or fabricated.
- Sign-in spends the same CPU on an unknown username as on a wrong password, by
  verifying against a reference hash computed once at startup rather than a
  pasted-in literal that cannot follow parameter changes.
- Registration relies on the unique index rather than a preceding `SELECT`,
  which removes both a round trip and the race between check and insert.
