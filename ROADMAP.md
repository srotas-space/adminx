# 🗺️ Admix Roadmap

[![Crates.io](https://img.shields.io/crates/v/admix.svg)](https://crates.io/crates/admix)
[![Docs.rs](https://docs.rs/admix/badge.svg)](https://docs.rs/admix)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Contributions Welcome](https://img.shields.io/badge/contributions-welcome-blue.svg)](https://github.com/srotas-space/admix/issues)
[![Project Status](https://img.shields.io/badge/status-actively--developed-brightgreen.svg)](https://github.com/srotas-space/admix)

> **Admix** is a minimal, declarative admin dashboard framework for Rust (Actix + MongoDB).

This roadmap tracks all major features planned — organized by category and development phase.

---

## ✅ Legend

- ✅ Completed  
- 🚧 In Progress  
- ⏳ Planned  

---

## Phase 1 – Core Foundation (MVP)

- ✅ Minimal setup with `AdmixDashboard::new().register::<Model>()`
- ✅ MongoDB integration (collection + ObjectId support)
- ✅ Auto route mounting for registered models (`/admin/{resource}`)
- ✅ Auto CRUD handlers: List, Show, Create, Update, Delete
- ✅ Pagination (`?page=1&per_page=25`)
- ✅ Dynamic filters from query params
- ✅ Sorting (`?sort=name`, `?sort=-created_at`)
- ⏳ Error handling and API responses
- ⏳ Trait or macro support for model metadata (`#[admix_resource]`)
- ⏳ Admin resource registration via macro or trait

---

## Phase 2 – Extended Resource Features

- ⏳ Field whitelisting (`permit_params`)
- ⏳ Read-only fields
- ⏳ Custom field rendering (formatters)
- ⏳ Enum/Select filter support
- ⏳ Date range filter support
- ⏳ Custom scopes (e.g., `scope :published`)
- ⏳ Inline/nested form support (`has_many`)
- ⏳ Belongs-to / polymorphic relationships
- ⏳ Batch actions (bulk delete, approve, etc.)

---

## Phase 3 – Auth & Permissions

- ⏳ Admin user login/authentication
- ⏳ Role-based access control (RBAC)
- ⏳ Conditional visibility for menus and fields
- ⏳ Per-resource or per-action permissions

---

## Phase 4 – Export & Import

- ⏳ CSV export of any index page
- ⏳ JSON export
- ⏳ Bulk import via CSV
- ⏳ Export field customization

---

## Phase 5 – Custom Pages & Actions

- ⏳ Admin dashboard (`/admin`) with widgets
- ⏳ Custom widgets/panels (e.g., resource stats)
- ⏳ Add custom buttons (approve, verify, etc.)
- ⏳ Add custom non-resource pages (e.g., reports)
- ⏳ Extend controller actions per resource

---

## Phase 6 – UI, Themes & Frontend

- ⏳ Default UI built in React/Yew (`admix-ui`)
- ⏳ Light/dark theme toggle
- ⏳ Custom logo and admin title
- ⏳ Per-resource customization (columns, labels)
- ⏳ Embedded HTML field rendering
- ⏳ JS/CSS injection per page

---

## Phase 7 – Internationalization

- ⏳ Full I18n support
- ⏳ Language toggle
- ⏳ Date/number localization

---

## Optional Extensions

- ⏳ Comments module
- ⏳ Tags support
- ⏳ Notifications & audit logs
- ⏳ Scheduled jobs (via cron or workers)
- ⏳ Soft delete / archiving

---

## Testing

- ⏳ Integration tests for all CRUD handlers
- ⏳ Pagination + filter test coverage
- ⏳ Auth and permission unit tests
- ⏳ CLI utility for testing fixtures

---

## Internal Utilities

- ✅ `filters.rs`: parse query into MongoDB filter
- ✅ `pagination.rs`: handle `page`, `per_page`
- ⏳ `mongo.rs`: collection helpers, ID parsing
- ⏳ `resource.rs`: base trait for auto generation
- ⏳ `generator.rs`: dynamic handler + route builder
- ⏳ `macros.rs`: `#[admix_resource]` derive macro

---

## 📬 Contributing

We ❤️ contributions!  
- [Open an issue](https://github.com/srotas-space/admix/issues) for bugs or feature requests  
- [Start a discussion](https://github.com/srotas-space/admix/discussions) to share ideas  
- [Submit a PR](https://github.com/srotas-space/admix/pulls) to help build the future of Admix  

---

> ✅ Marked items are completed  
> 🚧 Items in active development  
> ⏳ Items in upcoming phases
