//! Demo: load an INI file into memory, read a few typed values, then modify
//! and add entries.
//!
//!   cargo run --example ini_demo -- tests/fixtures/sample.ini
//!
//! Author: Anand <truchipinfo@gmail.com>

use std::process::exit;

use rustiniparser::Document;

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: ini_demo <file.ini>");
            exit(2);
        }
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot open {path}: {e}");
            exit(1);
        }
    };

    let mut doc = match Document::parse(&text) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("parse error: {e}");
            exit(1);
        }
    };

    println!("loaded {} entries", doc.len());

    println!("host    = {}", doc.get_string("network", "host", "<none>"));
    println!("port    = {}", doc.get_int("network", "port", 0));
    println!("timeout = {}", doc.get_int("network", "timeout", -1));
    println!("enabled = {}", doc.get_bool("server", "enabled", false));

    doc.set_int("network", "port", 9090).unwrap();
    doc.set("network", "gateway", "10.0.0.1").unwrap();

    println!("--- after modify/add ---");
    println!("port    = {}", doc.get_int("network", "port", 0));
    println!("gateway = {}", doc.get_string("network", "gateway", "<none>"));
    println!("entries = {}", doc.len());
}
