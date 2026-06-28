# Changelog

## V1.0.1 — 2026-06-28

- Added a stable **C ABI** so the library can be linked from C and C++.
  `cargo build --release` now also produces `librustiniparser.a` (staticlib)
  and `librustiniparser.so`/`.dll`/`.dylib` (cdylib); the `[lib] crate-type` is
  `["rlib", "staticlib", "cdylib"]`.
- New C-ABI surface in `src/ffi.rs` with the matching header
  `include/rustiniparser.h`: `ini_new`/`ini_parse`/`ini_load`/`ini_free`,
  typed getters, setters, `ini_has`, `ini_remove` and `ini_len`, plus
  `INI_OK`/`INI_ERR_*` status codes.
- Added runnable C and C++ samples (`examples/c`, `examples/cpp`) and an
  assertion-based FFI test harness with a `Makefile` (`tests/ffi`).
- The pure-Rust core remains free of `unsafe`; the only `unsafe` is confined to
  the FFI layer. (Crate-level `#![forbid(unsafe_code)]` was relaxed accordingly.)

## V1.0.0 — 2026-06-27

- Initial release.
- In-memory INI store: `Document::load`/`parse`, typed getters
  (string/int/double/bool), and mutation (`set`, `set_int`, `set_bool`,
  `remove`).
- Supports `[sections]` + global section, `=`/`:` separators, `;`/`#`
  comments (incl. inline), quoted values with escapes, empty values, and
  LF/CRLF/CR line endings.
- Allocation-free line scanning; optional `Limits` for fixed-capacity behaviour.
- Zero dependencies; `#![forbid(unsafe_code)]`; unit, integration and doc tests.
- Speed (`release`) and size (`min-size`) build profiles; cross-compiles cleanly.
