# AGENTS

## Scope
Panduan umum kontribusi untuk workspace `frontend`, `backend`, dan `lib`.

## Rules
- Gunakan Rust 1.97.1 dan Edition 2024.
- Jaga kontrak data bersama di `lib` agar frontend dan backend konsisten.
- Frontend Leptos menggunakan mode CSR via Trunk (`trunk build` / `trunk serve`) dengan target `wasm32-unknown-unknown`.
- Prasyarat frontend E2E: Node.js dan Playwright (test di `frontend/tests/e2e`).
- Jalankan `cargo fmt --all`, `cargo test --workspace --exclude frontend`, `trunk build` (di `frontend/`), dan Playwright E2E frontend sebelum merge.
- Buat perubahan kecil, terfokus, dan terdokumentasi.
- **Aturan Screenshot Frontend**: Setiap kali ada perubahan pada bagian frontend (UI/UX), wajib menampilkan tangkap layar (screenshot) sebelum ("before") dan sesudah ("after") perubahan tersebut di dokumentasi atau saat pull request.
- **Struktur Backend (Axum)**: Routing HTTP dipusatkan di `backend/src/routes.rs` (fungsi `app()` yang merakit `Router`). Logika handler dipisah per-domain di `backend/src/handlers/` dan hanya mengurus concern HTTP. Seluruh akses database diletakkan di lapisan `backend/src/repository/` per tabel/domain — handler tidak menulis SQL langsung. Infrastruktur (`state.rs`, `db.rs`, `extractors.rs`) berada di modul terpisah. Endpoint baru wajib didaftarkan di `routes.rs`, dengan handler di `handlers/` dan query di `repository/`.
