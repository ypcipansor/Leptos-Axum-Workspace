# Runbook

Operating this application in production.

## Before the first deploy

Two settings change security behaviour and have no safe default:

**`APP_ENV=production`.** Enables `Secure` on the session cookie, the `__Host-`
cookie prefix, HSTS, and JSON logging. Leaving it at `development` ships session
cookies without `Secure`, which means they travel over plain HTTP if anything
ever downgrades the connection.

Verify:

```bash
curl -si https://your-host/signin | grep -i strict-transport-security
```

**`TRUSTED_PROXY_HOPS`.** Must equal the number of reverse proxies actually in
front of the process.

| Deployment | Value |
| --- | --- |
| Container reached directly | `0` |
| Behind one nginx / Caddy / ALB | `1` |
| Behind a CDN in front of that | `2` |

Setting it higher than reality lets a client choose the address recorded against
its own sessions, by supplying its own `X-Forwarded-For`. The default `0` ignores
the header entirely and records the TCP peer.

Verify by signing in and checking the address shown in the session list is the
real client address, not the proxy's.

## Health probes

| Endpoint | Meaning | Wire it to |
| --- | --- | --- |
| `GET /health/live` | The process is running. Never touches the database. | Liveness / restart policy |
| `GET /health/ready` | The database is reachable. | Readiness / load-balancer routing |

The distinction matters: restarting a healthy process because its database is
briefly unreachable turns a recoverable blip into an outage. Point restart
policies at `/health/live` only.

`/health/ready` returns `200` with `{"database":"up",...}` or `503` with
`"down"`.

## Deploying

Migrations run automatically at startup, before the listener opens, under a
Postgres advisory lock — several replicas booting at once is safe, and an
instance never serves traffic against a schema it does not understand.

A rolling deploy therefore needs migrations to be backwards compatible with the
outgoing version, because both run briefly. For a breaking change, use the
expand/contract sequence:

1. Deploy a migration that **adds** the new column or table, nullable.
2. Deploy code that writes both old and new.
3. Backfill.
4. Deploy code that reads only the new.
5. Deploy a migration that **drops** the old.

Shutdown is graceful on `SIGTERM` and `SIGINT`: in-flight requests finish before
the process exits. Allow at least 30 seconds of termination grace, matching the
request timeout.

## Common problems

### The process will not start

Configuration is validated at boot and the error names the variable.

| Message | Cause |
| --- | --- |
| `required environment variable DATABASE_URL is not set` | Not set, or set to an empty string — an empty value is treated as unset, because orchestrators routinely inject `KEY=` |
| `environment variable APP_ENV is invalid` | Not `development` or `production`. Rejected rather than defaulted, because a typo would silently ship insecure cookie settings |
| `failed to apply database migrations` | The database is unreachable, or a migration checksum changed — see below |

### Migration checksum mismatch

sqlx records a hash of every applied migration. Editing a file that has already
run makes startup fail. This is intended: the deployed schema no longer matches
what the file says.

Never edit an applied migration. Add a new one that makes the change.

### Everyone is signed out after a deploy

Check whether `APP_ENV` changed. The cookie name differs between environments —
`__Host-session` in production, `session` in development — so a session
established under one is invisible to the other. This is deliberate: it stops a
development cookie authenticating against production.

### Sign-ins are being rejected as rate limited

The limiter allows 10 failed attempts per 5 minutes per (address, username).
A successful sign-in clears the count, so ordinary users are not affected.

Two situations produce false positives:

- **Many users behind one NAT address**, all mistyping. The key includes the
  username, so this only affects users sharing both an address and an account.
- **`TRUSTED_PROXY_HOPS=0` behind a proxy**, which makes every request appear to
  come from the proxy's address. Fix the setting rather than the limiter.

The budget lives in memory, so a restart clears it. Adjust `MAX_ATTEMPTS` and
`WINDOW` in `crates/domain/src/auth/rate_limit.rs`.

### The sessions table is growing

The sweep runs every `SESSION_CLEANUP_INTERVAL_SECONDS` (default hourly) and logs
`expired sessions removed`. If those lines are absent, the task has stopped.
Expired sessions are already unusable — every lookup filters on
`expires_at > now()` — so this is a storage problem, not a security one.

Clean up manually with:

```sql
DELETE FROM sessions WHERE expires_at <= now();
```

### Investigating one request

Every response carries `x-request-id`, and every log line for that request
carries the same value.

```bash
# Ask the reporter for the header value, then:
kubectl logs deploy/app | grep '"request_id":"<value>"'
```

In production the logs are JSON with one object per event.
`RUST_LOG=debug,sqlx=info` raises the level; the default is
`info,sqlx=warn,hyper=warn,tower_http=info,h2=warn`.

## Rotating a compromised session

Sessions are revocable individually because the store is queryable — this is the
practical payoff of the design in
[ADR 0002](adr/0002-opaque-database-backed-sessions.md).

```sql
-- One user, everywhere. They will be signed out on their next request.
DELETE FROM sessions
WHERE user_id = (SELECT id FROM users WHERE lower(username) = lower('alice'));

-- Everyone.
TRUNCATE sessions;
```

There is no cache to invalidate and no token that stays valid until it expires;
the next request from a deleted session fails to authenticate.

## Raising the Argon2 cost

Parameters are embedded in each stored PHC string, so raising them does not
invalidate existing hashes — old passwords keep verifying at their original cost
and get the new one when next changed.

Edit the `Argon2::default()` call in `crates/domain/src/auth/password.rs`, then
measure: hashing should take roughly 100 ms on production hardware. Much faster
and it is too cheap to attack; much slower and sign-in becomes a denial-of-service
vector against yourself.

## Backups

The only durable state is Postgres. `sessions` is disposable — losing it signs
everyone out, nothing worse. `users` is not.
