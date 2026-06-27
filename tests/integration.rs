//! Integration tests driving the public API against the bundled fixtures.

use rustiniparser::{Document, Error};

fn load_fixture(name: &str) -> Document {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).expect("fixture present");
    Document::parse(&text).expect("fixture parses")
}

#[test]
fn sample_fixture_typed_reads() {
    let doc = load_fixture("sample.ini");

    assert_eq!(doc.get_string("", "app_name", "?"), "Rustiniparser");
    assert_eq!(doc.get_double("", "version", 0.0), 1.0);

    assert_eq!(doc.get_string("network", "host", "?"), "example.com");
    assert_eq!(doc.get_int("network", "port", 0), 8080);
    assert_eq!(doc.get_int("network", "timeout", 0), 30);
    assert_eq!(doc.get_int("network", "mask", 0), 255);
    // empty value
    assert_eq!(doc.get_string("network", "proxy", "<unset>"), "");

    assert!(doc.get_bool("server", "enabled", false));
    assert_eq!(doc.get_string("server", "log_level", "?"), "debug");
    // single quotes preserve `;` and `#`
    assert_eq!(
        doc.get_string("server", "password", "?"),
        "p@ss #word ; not-a-comment"
    );
}

#[test]
fn modify_add_remove_roundtrip() {
    let mut doc = load_fixture("sample.ini");
    let before = doc.len();

    doc.set_int("network", "port", 9090).unwrap();
    assert_eq!(doc.get_int("network", "port", 0), 9090);
    assert_eq!(doc.len(), before, "overwrite must not grow the document");

    doc.set("network", "gateway", "10.0.0.1").unwrap();
    assert_eq!(doc.len(), before + 1);

    doc.remove("network", "gateway").unwrap();
    assert_eq!(doc.remove("network", "gateway"), Err(Error::NotFound));
    assert_eq!(doc.len(), before);
}

#[test]
fn merging_two_buffers() {
    let mut doc = Document::new();
    doc.load("[a]\nx = 1\n").unwrap();
    doc.load("[b]\ny = 2\n").unwrap();
    assert_eq!(doc.get_int("a", "x", 0), 1);
    assert_eq!(doc.get_int("b", "y", 0), 2);
}
