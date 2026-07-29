# Leptos Axum Workspace Template

Template aplikasi web **Sistem Informasi Manajemen sederhana** berbasis Rust workspace, siap pakai dan mudah dikembangkan.

## Stack

- **Rust** 1.97.1 (Edition 2024)
- **Frontend**: Leptos 0.8.19
- **Backend**: Axum 0.8.9
- **Shared library**: crate bersama untuk model/konstanta lintas frontend-backend

## Struktur Proyek

```text
.
├── frontend/   # Aplikasi frontend Leptos
├── backend/    # API backend Axum
├── lib/        # Shared library untuk type/model bersama
└── .github/    # Workflow CI, template issue/PR, CODEOWNERS, Dependabot
```

## Menjalankan

```bash
cargo test --workspace
cargo run -p backend
cargo run -p frontend
```

## Best-practice yang diterapkan

- Monorepo Cargo workspace untuk konsistensi dependency
- Shared type lintas service untuk mengurangi duplikasi kontrak data
- CI dasar (fmt, clippy, test)
- Dependabot & CODEOWNERS untuk maintainability
