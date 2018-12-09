//!
//! The GNU unidiff Rust binary entry point.
//!

extern crate unidiff;
extern crate chrono;

use std::{
    fs,
    io::{self, BufRead},
};

use chrono::{DateTime, Local};

fn read_file(path: &str) -> io::Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let file = io::BufReader::new(file);
    file.lines().collect()
}

fn timestamp(path: &str) -> io::Result<String> {
    let metadata = fs::metadata(path)?;
    let filetime: DateTime<Local> = DateTime::from(metadata.modified()?);
    Ok(filetime.format("%Y-%m-%d %H:%M:%S.%f %z").to_string())
}

fn main() -> io::Result<()> {
    const FILE_1: &'static str = "Cargo.lock";
    const FILE_2: &'static str = "Cargo.toml";
    const CONTEXT_RADIUS: usize = 3;

    let file1 = read_file(FILE_1)?;
    let file2 = read_file(FILE_2)?;

    println!("--- {}\t{}", FILE_1, timestamp(FILE_1)?);
    println!("+++ {}\t{}", FILE_2, timestamp(FILE_2)?);

    for s in unidiff::unidiff(&file1, &file2, CONTEXT_RADIUS)? {
        println!("{}", s);
    }
    Ok(())
}
