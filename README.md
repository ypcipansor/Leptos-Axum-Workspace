# Leptos Axum Workspace Template

Template aplikasi web **Sistem Informasi Manajemen sederhana** berbasis Rust workspace, siap pakai dan mudah dikembangkan.

## Stack

- **Rust** 1.97.1 (Edition 2024)
- **Frontend**: Leptos 0.8.19 (CSR + Trunk)
- **Backend**: Axum 0.8.9
- **Shared library**: crate bersama untuk model/konstanta lintas frontend-backend

## Struktur Proyek

```text
.
├── frontend/   # Aplikasi frontend Leptos (CSR via Trunk)
├── backend/    # API backend Axum
├── lib/        # Shared library untuk type/model bersama
└── .github/    # Workflow CI, template issue/PR, CODEOWNERS, Dependabot
```

## Prasyarat

- Rust 1.97.1
- Target wasm: `rustup target add wasm32-unknown-unknown`
- Trunk: `cargo install --locked trunk`
- Node.js (untuk E2E frontend)
- Playwright browser deps: `npx playwright install --with-deps`

## Menjalankan

```bash
# Backend
cargo run -p backend

# Frontend (jalankan dari folder frontend)
cd frontend
trunk serve
```

## Menjalankan Test

```bash
# Native checks (backend + lib)
cargo fmt --all -- --check
cargo clippy --workspace --exclude frontend --all-targets -- -D warnings
cargo test --workspace --exclude frontend --all-targets

# Frontend wasm build
cd frontend
trunk build

# Frontend E2E (Playwright)
cd tests/e2e
npm ci
npx playwright install
npm run e2e

# Backend E2E
cargo test -p backend
```

## Best-practice yang diterapkan

- Monorepo Cargo workspace untuk konsistensi dependency
- Shared type lintas service untuk mengurangi duplikasi kontrak data
- CI dasar (native checks + frontend wasm build + frontend e2e)
- Dependabot & CODEOWNERS untuk maintainability
