# Changelog

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
