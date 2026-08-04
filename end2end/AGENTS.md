# `end2end`

Playwright suites. These run against a real `cargo leptos serve`, so they
exercise the release build rather than a development stand-in.

## The projects

| Project | Purpose |
| --- | --- |
| `chromium`, `firefox` | The main suites on two engines |
| `mobile-chrome` | Catches layout that only breaks on a narrow viewport |
| `no-javascript` | Runs `no-js.spec.ts` with scripting disabled |

The `no-javascript` project is the one that verifies the architecture actually
delivers what it claims. Every test in it would fail against a client-rendered
build, where a page without JavaScript is an empty `<body>` and no form can be
submitted. If you are tempted to delete it because it seems redundant, it is not:
nothing else checks that server rendering still works end to end.

## Writing tests

**Select by role and label**, not by CSS class or test id:

```ts
await page.getByLabel('Username').fill(username);
await page.getByRole('button', { name: 'Sign in' }).click();
```

This is not only more readable — it fails when a control loses its accessible
name, which is a defect worth failing on.

**Use `uniqueUsername()`** from `support.ts`. The suites share a database, so a
fixed name makes the second run fail against leftovers from the first.

**A second signed-in user needs `browser.newContext()`.** A new page in the same
context shares the cookie jar, so it shares the session.

## Accessibility

`a11y.spec.ts` scans every page with axe-core in both light and dark themes.
Contrast is the failure mode a second theme introduces most often, and it is
invisible to anyone testing only the default one.

A new page should get an entry there. A violation is a build failure, not a
warning.
