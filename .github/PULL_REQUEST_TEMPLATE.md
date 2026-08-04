## What changed

Describe the change and, more importantly, why it was needed.

## How it was verified

- [ ] `just ci` passes locally
- [ ] `just e2e` passes, or the change cannot affect the browser

Note anything a reviewer should look at closely.

## If applicable

- [ ] Schema change ships with a reversible migration, and no applied migration was edited
- [ ] `just prepare` was run after changing a query, and `.sqlx` is committed
- [ ] New behaviour has a test that fails without the change
- [ ] UI change keeps labels, live regions and keyboard access intact
- [ ] Security-relevant behaviour is called out explicitly below

## Notes

Anything else worth knowing: a decision you were unsure about, a follow-up you
deliberately left out, an alternative you rejected.
