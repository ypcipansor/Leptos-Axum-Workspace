# `app-core`

The contract between the browser and the server: validated newtypes, wire DTOs,
and the error enum the UI renders.

## Compiles to wasm

Every dependency here ends up in the browser bundle. Nothing in this crate may
touch the filesystem, the clock, or randomness, and new dependencies should be
weighed against the download they add.

If you need `Utc::now()`, you are in the wrong crate.

## Parse, do not validate

Rules live inside the constructors — `Username::parse`, `Password::parse` — and
nowhere else. Deserialization routes through them via
`#[serde(try_from = "String")]`, so an invalid value cannot enter the process
even from a crafted payload.

This is why validation cannot drift: the browser calls the same function the
server does, and there is no second implementation to fall out of step.

Adding a rule means editing the parser. Adding a check in a handler recreates the
duplication this crate exists to remove.

## Secrets

`Password` has a hand-written `Debug` that prints `Password(<redacted>)`. Any
new type carrying a secret needs the same, and a test asserting it.

## Wire compatibility

Everything here is serialized across the network and, for resources, embedded in
server-rendered HTML for hydration to pick up. Renaming a field or a variant is a
breaking change for any client mid-session. Round-trip tests guard the shapes;
keep them when you add a type.
