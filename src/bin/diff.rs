//!
//! The GNU unidiff Rust binary entry point.
//!

extern crate unidiff;

fn main() {
    const CONTEXT_RADIUS: usize = 3;

    match unidiff::unidiff("Cargo.lock", "Cargo.toml", CONTEXT_RADIUS) {
        Ok(data) => for s in data {
            println!("{}", s);
        },
        Err(error) => {
            eprintln!("{}", error);
        }
    }
}
