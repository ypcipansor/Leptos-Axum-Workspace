# 3. Five-crate workspace layout

- **Status:** Accepted
- **Date:** 2026-08-04

## Context

The workspace was `frontend/`, `backend/` and `lib/`. `lib/` held bare DTOs;
everything else lived in one of the two application crates. With server-side
rendering (ADR 0001) that shape stops working, for a reason that is not a
preference but a hard constraint:

**`#[server]` functions must be defined in the crate that also renders the UI**,
because the macro emits the client-side call and the server-side handler from the
same definition. Their bodies need database access. If that access lives in the
binary crate, then the UI crate depends on the binary and the binary depends on
the UI crate — which Cargo rejects.

## Decision

Five crates, with dependencies flowing one way:

```
core ← domain ← app ← { frontend, server }
```

| Crate | Contains | Compiled for |
| --- | --- | --- |
| `app-core` | Validated newtypes, DTOs, the shared error enum | native **and** wasm |
| `app-domain` | Config, migrations, repositories, auth, telemetry | native |
| `app` | Components, routes, every `#[server]` function | native (`ssr`) **and** wasm (`hydrate`) |
| `frontend` | The wasm entrypoint — about fifteen lines | wasm |
| `server` | Axum router, middleware, health, shutdown | native |

`app-domain` exists to break the cycle. Its side effect is the more valuable
property: **it carries no Leptos dependency at all**, so the entire backend is
plain Axum/sqlx/Tokio and the UI layer can be replaced without touching it. This
is the concrete mitigation for the framework lock-in discussed in ADR 0001, and
it is enforced by the crate graph rather than by convention.

`app-core` is compiled to wasm, so it must stay free of filesystem, clock and
randomness dependencies. This is what allows `Username::parse` to run in the
browser for live feedback and on the server before the insert — one definition of
the rule, no second copy to drift.

## Feature handling

`app` has mutually exclusive `ssr` and `hydrate` features. Rather than passing
them on the command line, each consumer pins the one it needs in its own
manifest:

```toml
# crates/server/Cargo.toml
app = { workspace = true, features = ["ssr"] }

# crates/frontend/Cargo.toml
app = { workspace = true, features = ["hydrate"] }
```

Neither binary can therefore be built with the wrong half of the pair, and
`cargo check -p server` behaves identically to a `cargo leptos` build. Because
the two are only ever compiled for different targets — `frontend` is a `cdylib`
built for `wasm32-unknown-unknown` — the features never unify.

The consequence is that `cargo clippy --workspace` cannot lint everything in one
invocation. CI runs two commands instead, which is also what finally gets the
browser-side Rust linted: the previous workflow passed `--exclude frontend` to
every check, so none of it ever was.

## Consequences

Five crates is more than three, and a newcomer has to learn where things live.
The layering is strict enough to be mechanical, though: SQL only in `repo`,
policy only in `auth`, no Leptos below `app`, nothing platform-specific in
`core`.

Incremental builds improve, since editing a component does not rebuild the
database layer.
