# 1. Server-rendered Leptos, integrated with Axum

- **Status:** Accepted
- **Date:** 2026-08-04

## Context

The repository described itself as a Leptos + Axum template, but the two halves
were separate applications that happened to share a workspace: a client-rendered
SPA served as static files behind nginx, and an Axum REST API in another
container.

Nothing that distinguishes the combination was in use — no `leptos_axum`, no
server functions, no `leptos_router`, no `leptos_meta`, no `cargo-leptos`. The
shared crate held bare DTOs. The client talked to the server through string URL
literals and hand-written `serde_json`, routing was a signal holding an enum, and
validation rules existed twice with nothing keeping them in step.

Six options were evaluated against six weighted criteria.

| Option | Score |
| --- | --- |
| **A1 — Full SSR with hydration** | **4.30** |
| A3 — SSR plus a public versioned REST API | 4.25 |
| A2 — Islands architecture | 3.85 |
| A6 — SSR UI and a separate API service | 3.15 |
| A4 — Client-rendered SPA calling server functions | 2.85 |
| A5 — Client-rendered SPA and REST, modernised | 2.50 |

## Decision

Adopt **A1**: `cargo-leptos`, `leptos_axum`, `leptos_router`, `leptos_meta`, and
`#[server]` functions, served from a single binary.

## Consequences

**What this buys.** Server functions remove the entire hand-written client layer;
the network boundary becomes type-checked, so a signature change breaks both
sides at compile time rather than at runtime. Pages arrive as complete HTML,
which makes them indexable and usable without JavaScript. Deployment collapses
from two containers plus nginx to one image.

**What it costs.** The UI crate compiles for two targets, so every dependency it
touches must be wasm-safe. `cargo leptos` becomes a required build tool.

**Why not the alternatives.**

*A3* differs from A1 only by also exposing `/api/v1` with OpenAPI. It can be
added on top of A1 at any time, because the domain layer is already separate.
Adding it now would mean maintaining a second API surface, and a second
authentication path, for consumers that do not exist.

*A2 (islands)* scores well and produces a far smaller wasm payload. It is also
the youngest feature in a framework that is now lightly maintained, with the
fewest worked examples, and it makes auth-aware UI meaningfully harder. A1 is a
prerequisite for A2 rather than a competitor: moving to islands later means
adding `#[island]` annotations, not changing architecture.

*A5* was the lowest-risk option and the one that answers the original request
least — it leaves every distinguishing feature of the stack unused.

## Managing the lock-in

Leptos was declared feature-complete and lightly maintained in
[May 2026](https://github.com/leptos-rs/leptos/issues/4707). For a foundation
template, a stable API that will not churn is largely an asset, and bug-fix
releases have continued. The risk is that a future need the framework does not
cover becomes ours to solve.

The mitigation is structural: `crates/domain` has **no Leptos dependency**.
Configuration, migrations, repositories, authentication and telemetry are plain
Axum, sqlx and Tokio. Replacing the UI layer means replacing `crates/app` and
`crates/frontend`; roughly 60% of the code is unaffected. This is enforced by
the crate graph, not by convention.

## Note: tower-http stays at 0.6

`leptos_axum` 0.8 depends on `tower-http` 0.6 while 0.7 is current. Using 0.7 in
the server crate would pull a second, semver-incompatible copy into the tree for
no functional gain, so the workspace pins 0.6. Revisit when `leptos_axum`
updates.
