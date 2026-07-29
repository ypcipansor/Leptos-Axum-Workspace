# AGENTS - Frontend

## Scope
Panduan untuk crate `frontend` (Leptos 0.8.19, CSR via Trunk).

## Rules
- Frontend wajib CSR (`leptos` feature `csr`) dan dijalankan dengan Trunk (`trunk serve` / `trunk build`), bukan `cargo run`.
- Pastikan target `wasm32-unknown-unknown` tersedia.
- Bootstrap aplikasi via `mount_to_body(App)` (hindari `view!` langsung di `main`).
- Gunakan struktur berikut sebagai baseline:
  - `src/main.rs`, `src/lib.rs`, `src/app.rs`
  - `src/routes.rs` untuk sumber rute bertipe tunggal
  - `src/features/<domain>/` untuk page/state/api khusus domain
  - `src/components/` hanya untuk komponen dumb/shared lintas fitur
  - `src/utils/` hanya helper non-domain
  - `tests/e2e/` untuk Playwright E2E
- Simpan shared type/konstanta di `../lib` agar kontrak frontend-backend konsisten.
