# `frontend`

The wasm entrypoint. It should stay about fifteen lines.

Its only job is to install a panic hook and hand the already-rendered DOM to
`app`. Components, routes and server functions belong in `crates/app`, which is
what makes them available to the server build as well.

## Why this is a separate crate

It must be a `cdylib` for wasm-bindgen, and it pins `app`'s `hydrate` feature in
its manifest — the mirror of what `server` does with `ssr`. Because the two are
only ever compiled for different targets, the mutually exclusive features never
unify.

## Linting

`cargo clippy --workspace` cannot cover this crate; it needs its own invocation
against `wasm32-unknown-unknown`, which `just lint` and CI both run.

This is worth knowing because the previous workflow passed `--exclude frontend`
to every check, so none of the browser-side Rust was ever linted at all.
