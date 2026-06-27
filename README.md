# Rustiniparser

A tiny **in-memory INI store** for Rust. You feed it INI text, it parses
everything into a `Document`, and then you:

- **read values back** converted to the type you want — string, int, double, bool;
- **modify** existing fields;
- **add** new entries (and remove them).

It uses `std` collections, so a document grows as needed and keys/values may be
of any length. Fixed-capacity behaviour is available opt-in via `Limits`. The
parser does no allocation while scanning lines and allocates only the strings it
stores, so loading is fast. Zero dependencies, `#![forbid(unsafe_code)]`.

---

## At a glance

```rust
use rustiniparser::Document;

let text = "\
[network]
host = example.com
port = 8080
enabled = yes
";

let mut doc = Document::parse(text).unwrap();

let host    = doc.get_string("network", "host", "localhost");
let port     = doc.get_int   ("network", "port", 80);
let enabled = doc.get_bool  ("network", "enabled", false);

doc.set_int("network", "port", 9090).unwrap();        // modify existing
doc.set("network", "gateway", "10.0.0.1").unwrap();   // add new
```

---

## Using it

Add to `Cargo.toml`:

```toml
[dependencies]
rustiniparser = { path = "." }   # or a version/git source
```

Build and test:

```sh
cargo build              # build the library
cargo test               # unit + integration + doc tests
cargo run --example ini_demo -- tests/fixtures/sample.ini
```

### Debug build

The default build is the `dev` profile: unoptimized, with full debug info,
debug assertions and integer-overflow checks on — best for development and
stepping through in a debugger:

```sh
cargo build                       # debug build (target/debug/)
cargo test                        # tests also run under the dev profile
```

### Optimized builds

Two tuned profiles are configured in `Cargo.toml` — both use fat LTO, a single
codegen unit, `panic = "abort"` and symbol stripping:

```sh
cargo build --release             # optimized for speed (opt-level 3)
cargo build --profile min-size    # optimized for size  (opt-level "z")
```

### Cross-compilation

Being pure Rust with no C dependencies, the crate cross-compiles cleanly. The
general recipe is:

```sh
rustup target add <triple>                       # 1. install the target's std
cargo build --release --target <triple>          # 2. build (add --profile min-size for size)
```

For example, for a 64-bit ARM Linux device with a static musl binary:

```sh
rustup target add aarch64-unknown-linux-musl
cargo build --release --target aarch64-unknown-linux-musl
```

#### Common targets

| Triple | Platform | Cross linker needed |
|---|---|---|
| `aarch64-unknown-linux-gnu` | 64-bit ARM Linux (glibc) | `aarch64-linux-gnu-gcc` |
| `aarch64-unknown-linux-musl` | 64-bit ARM Linux (static) | `aarch64-linux-musl-gcc` |
| `armv7-unknown-linux-gnueabihf` | 32-bit ARMv7 (Raspberry Pi, etc.) | `arm-linux-gnueabihf-gcc` |
| `x86_64-unknown-linux-musl` | x86-64 Linux (static) | — (works out of the box) |
| `x86_64-pc-windows-gnu` | 64-bit Windows | `x86_64-w64-mingw32-gcc` |

If a target needs a cross linker, uncomment the matching entry in
[.cargo/config.toml](.cargo/config.toml), e.g.:

```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

#### Using `cross` (no host toolchain setup)

The easiest path for most targets is
[`cross`](https://github.com/cross-rs/cross), which supplies the C toolchains in
containers so you don't have to install them yourself:

```sh
cargo install cross
cross build --release --target aarch64-unknown-linux-gnu
```

> **Note:** `no_std` bare-metal targets (e.g. `thumbv7em-none-eabihf`) need an
> allocator; the crate currently depends on `std` for its collections.

---

## Accepted INI syntax

- `[sections]`, plus a **global section** (`""`) for keys before any header
- `key = value` and `key : value`
- `;` and `#` comment lines, and inline comments after a value
- single (`'...'`) and double (`"..."`) quoted values (preserve spaces / comment chars)
- a few escapes inside double quotes: `\\ \" \n \r \t`
- empty values (`key =`)
- LF, CRLF and lone-CR line endings

Lookups are **case-sensitive** for section and key names.

---

## API reference

### Types

```rust
pub struct Document { /* ordered entries */ }
pub struct Entry { pub section: String, pub key: String, pub value: String }

pub enum Error {
    Syntax,    // malformed line in the input
    TooLong,   // section/key/value exceeded a configured limit
    Full,      // document already holds max_entries
    NotFound,  // key not present (remove)
}
```

### Lifecycle

| Method | Description |
|---|---|
| `Document::new()` | Empty document, no limits. |
| `Document::with_limits(limits)` | Empty document enforcing [`Limits`]. |
| `Document::parse(&str) -> Result<Document>` | Parse a buffer in one step. |
| `doc.load(&str) -> Result<()>` | Parse a buffer and append its entries (call again to merge). |
| `doc.clear()` / `doc.len()` / `doc.is_empty()` / `doc.entries()` | Inspect / reset. |

### Getters

Each takes a `section` (use `""` for global) and returns `fallback` when the
key is missing (or, for typed getters, unconvertible).

| Method | Returns |
|---|---|
| `has(section, key) -> bool` | Whether the key exists. |
| `get_string(section, key, fallback) -> &str` | Raw string value. |
| `get_int(section, key, fallback) -> i64` | Decimal, or `0x`-prefixed hex. |
| `get_double(section, key, fallback) -> f64` | Floating point (supports `e` exponent). |
| `get_bool(section, key, fallback) -> bool` | Accepts `true/false`, `yes/no`, `on/off`, `1/0` (case-insensitive). |

### Setters / mutation

All update an existing key in place, or add a new entry if it is absent.

| Method | Description |
|---|---|
| `set(section, key, value)` | Set a string value. |
| `set_int(section, key, i64)` | Set an integer (stored as decimal text). |
| `set_bool(section, key, bool)` | Store `"true"` / `"false"`. |
| `remove(section, key)` | Delete a key; `Error::NotFound` if absent. |

Setters return `Error::Full` / `Error::TooLong` only when [`Limits`] are
configured; a default document never fails on a set.

---

## Optional limits (fixed capacity)

To cap the document and reject oversized input:

```rust
use rustiniparser::{Document, Limits};

let mut doc = Document::with_limits(Limits::COMPACT);
// or fully custom:
let mut doc = Document::with_limits(Limits {
    max_entries: Some(128),
    ..Limits::default()
});
```

---

## Project layout

```
src/lib.rs              library (parser + typed store)
examples/ini_demo.rs    load / get / modify / add demo
tests/integration.rs    integration tests
tests/fixtures/         sample .ini files
```

---

## License

See [LICENSE](LICENSE).
