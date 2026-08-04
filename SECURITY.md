# Security policy

## Reporting a vulnerability

**Please do not open a public issue.**

Use GitHub's [private vulnerability reporting][pvr] on this repository, which
notifies the maintainers without disclosing the report.

Helpful to include: what an attacker can do, the steps to reproduce it, and the
version or commit you tested.

You can expect an acknowledgement within a few days and an assessment within two
weeks. If a fix is warranted we will agree a disclosure timeline with you, and
credit you in the release notes unless you prefer otherwise.

[pvr]: https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability

## Scope

This is a template. Anyone deploying it owns the security of their deployment.
Reports about **this codebase** are in scope; reports about a site built from it
should go to whoever runs that site.

Out of scope: findings that depend on a misconfiguration this project documents
against — most commonly running with `APP_ENV=development` in production, or
setting `TRUSTED_PROXY_HOPS` higher than the number of proxies actually present.
Both are called out in the [runbook](docs/runbook.md).

## What this template does

Deploy-time responsibilities are listed after the built-in measures, because
several of the guarantees below only hold if the deployment is configured
correctly.

### Authentication

- **Argon2id** password hashing at OWASP default parameters, on the blocking
  pool. Parameters are embedded in each stored hash, so they can be raised later
  without invalidating existing ones.
- **Opaque session tokens**: 256 bits from the OS CSPRNG. The database stores
  only a SHA-256 digest, so a leaked dump contains no usable credential.
- **`HttpOnly` cookies** with `SameSite=Lax`, plus `Secure` and the `__Host-`
  prefix in production. The token is never exposed to JavaScript and never
  returned by a server function.
- **Expiry** enforced in every query, so a stale session is unusable regardless
  of whether the cleanup sweep has run. The window slides forward on use.
- **Immediate revocation.** Deleting the row ends the session on the next
  request; there is no token that stays valid until it expires.
- **Rate limiting** on sign-in, keyed on address *and* username, so one account
  cannot be ground down from many addresses and a shared address does not lock
  out everyone behind it. Only failures count.
- **Uniform failure responses.** An unknown username and a wrong password return
  the same answer and spend the same CPU, so usernames cannot be enumerated by
  response or by timing.

### Data handling

- All queries are checked against the real schema at compile time and use bound
  parameters. The one place dynamic SQL is unavoidable — creating a test
  database — is wrapped in sqlx's `AssertSqlSafe` with the audit written down.
- Passwords, tokens and connection strings have redacting `Debug`
  implementations, so they cannot reach a log line by accident.
- Internal errors are collapsed to a generic message before crossing the network;
  the detail stays in the server log, correlated by request id.

### Transport

- Content-Security-Policy, `X-Content-Type-Options: nosniff`,
  `X-Frame-Options: DENY`, `Referrer-Policy`, `Permissions-Policy`, and HSTS in
  production.
- Request body limit (256 KiB) and request timeout (30 s).
- Client addresses come from the TCP peer unless `TRUSTED_PROXY_HOPS` declares
  how many proxies sit in front.

### Supply chain

CI runs `cargo-deny` (advisories, licences, banned crates, registry pinning) on
every push and weekly, so an advisory published against an unchanged dependency
is still found. Dependency review runs on pull requests, Dependabot keeps the
tree current, and GitHub Actions are pinned by commit SHA.

## Known limitation

The Content-Security-Policy includes `'unsafe-inline'` for `script-src`, because
Leptos emits its hydration bootstrap and this app its theme-init script as inline
`<script>` elements. Removing it requires enabling leptos's `nonce` feature and
threading `use_nonce()` through the shell. This is documented rather than left
silent; see [`docs/architecture.md`](docs/architecture.md).

`'wasm-unsafe-eval'` is also present. It permits WebAssembly compilation only,
not the far broader `'unsafe-eval'`.

## Your responsibilities when deploying

1. Set `APP_ENV=production`. Without it, session cookies are issued without
   `Secure`.
2. Set `TRUSTED_PROXY_HOPS` to the real number of reverse proxies in front of the
   process. Too high, and a client can choose the address recorded against its
   own sessions.
3. Terminate TLS. This application does not.
4. Keep `DATABASE_URL` out of version control and out of image layers.
5. Back up Postgres. It holds the only durable state.
