# AGENTS - Backend

## Scope
Panduan untuk crate `backend` (Axum 0.8.9).

## Rules
- Definisikan endpoint yang jelas dan stabil.
- Gunakan type dari `../lib` untuk response/request bersama.
- Pertahankan handler sederhana, validasi input, dan error handling eksplisit.
- Pertahankan E2E backend via `cargo test` integration test di `backend/tests/` yang menguji HTTP layer nyata.
