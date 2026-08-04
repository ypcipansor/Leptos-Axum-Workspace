# `app`

The user interface and every server function. Compiled twice: `ssr` on the
server, `hydrate` in the browser.

## The two builds

Anything reaching the database, the filesystem or Axum must sit behind
`#[cfg(feature = "ssr")]`, which is why the `ssr` module and the bodies of
`#[server]` functions are gated. Under `hydrate` those bodies are replaced by
network calls and the server code is not compiled at all.

Never add a dependency that fails to build for `wasm32-unknown-unknown` outside
the optional, `ssr`-only set.

## Server functions

Live in `server/auth.rs`. Each one:

- Declares an explicit `endpoint = "..."`. Without it the generated path carries
  a hash that changes whenever the signature does, breaking any cached client.
- Takes flat `String` arguments rather than nested structs, so `<ActionForm>` can
  post it as an ordinary HTML form. Parse into the `app-core` newtypes as the
  first statement in the body.
- Starts protected work with `ssr::require_user`.
- Converts domain failures with `from_domain`, which logs the detail and returns
  a safe `ApiError`. Never return an error carrying internal text.

Note the deliberate asymmetry between `sign_up` and `sign_in`: sign-up reports
precisely which rule a value broke, sign-in reports nothing specific. Telling a
signed-out stranger that a username is "too short to exist" confirms which names
are absent.

## Components

Every component owns its accessibility rather than leaving it to call sites:

- A real `<label for>`, never a placeholder standing in for one.
- `aria-invalid` when a field is in error, `aria-describedby` linking hint and
  error text.
- Live regions exist in the DOM before their content changes — mounting the
  element and its text together announces nothing.
- Decorative graphics carry `aria-hidden="true"`.

`just e2e` runs an axe-core scan over every page in both themes.

## Data flow

Use `Resource` for reads, `ServerAction` for writes, `Transition` for loading and
`ErrorBoundary` for failure. Do not fetch inside `Effect::new` + `spawn_local`:
it renders nothing on the server, so the page arrives empty and the whole point
of server rendering is lost.

To refresh after a mutation, make the resource depend on the action's version:

```rust
let sessions = Resource::new(move || revoke.version().get(), |_| list_sessions());
```

## Styling

Tailwind v4, configured in `style/main.css`. Reference the semantic tokens —
`bg-surface`, `text-body`, `border-subtle`, `text-accent` — rather than raw
colours, so both themes stay correct without a `dark:` variant on every element.

New utility classes must appear as literal strings; Tailwind scans the source and
cannot see a class name assembled at runtime.
