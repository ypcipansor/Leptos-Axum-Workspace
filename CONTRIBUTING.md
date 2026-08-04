# Contributing

Thanks for taking an interest. This document covers getting set up and what a
change needs before it can be merged.

## Setting up

You need [Rust](https://rustup.rs) (the toolchain is pinned in
`rust-toolchain.toml` and installs itself), a reachable Postgres, and:

```bash
cargo install just cargo-leptos sqlx-cli --locked
```

Then:

```bash
cp .env.example .env      # point DATABASE_URL at your Postgres
just migrate
just dev                  # http://127.0.0.1:3000
```

`just` on its own lists every task.

## Before you open a pull request

Run:

```bash
just ci
```

That is precisely what the pipeline runs, in the same order — format check,
clippy on native *and* wasm, tests, dependency audit, release build. If it
passes locally it will pass in CI.

Browser tests are separate because they need a running server:

```bash
just e2e
```

## What a change needs

**Tests.** New behaviour needs a test that fails without it. Integration tests
run against the real router and a real database; each gets its own database, so
you can add one without worrying about the others.

**A migration, if the schema changed.** `just migrate-new <name>`, fill in both
the `.up.sql` and the `.down.sql`, then `just migrate && just prepare`. Never
edit a migration that has already been applied — sqlx records a checksum and
startup will refuse.

**`just prepare`, if you changed a query.** The `.sqlx` cache is committed so
`cargo check` works without a database, and CI fails if it is stale.

**Accessibility, if you touched the UI.** Components own their own labelling and
live regions; `just e2e` runs an axe-core scan over every page in both themes.

See [`AGENTS.md`](AGENTS.md) for the layering rules and the security expectations
each layer has to preserve, and [`docs/adr/`](docs/adr/) for why the significant
decisions were made.

## Style

Commit messages in the imperative, with the reason in the body:

```
Reject X-Forwarded-For unless a proxy hop count is configured

The header is attacker-controlled, so trusting it unconditionally let a
client choose the address recorded against its own sessions.
```

Comments should explain *why*, not restate the code. The most useful ones answer
a question a reader would otherwise have to research: why a bound was chosen,
what breaks without a line, which alternative was rejected.

Everything is in English — code, comments, commits, docs and UI.

## Proposing something larger

For a change that alters architecture, adds a dependency, or changes security
behaviour, please open an issue first. Existing decisions and the alternatives
weighed against them are recorded in [`docs/adr/`](docs/adr/); if you are
proposing to revisit one, that is the place to start.

## Reporting a vulnerability

Do not open a public issue. See [`SECURITY.md`](SECURITY.md).
