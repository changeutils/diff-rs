//!
//! The GNU unidiff Rust binary entry point.
//!

extern crate diff_rs;

extern crate clap;
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
    let args = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            clap::Arg::with_name("file_1")
                .help("The first file to compare")
                .index(1)
                .value_name("PATH_1")
                .takes_value(true)
                .required(true))
        .arg(
            clap::Arg::with_name("file_2")
                .help("The second file to compare")
                .index(2)
                .value_name("PATH_2")
                .takes_value(true)
                .required(true))
        .arg(
            clap::Arg::with_name("context_radius")
                .help("The unidiff context radius")
                .short("c")
                .long("context")
                .value_name("NUMBER")
                .takes_value(true)
                .default_value("3"))
        .get_matches();

    let file_1 = args.value_of("file_1").unwrap();
    let file_2 = args.value_of("file_2").unwrap();
    let context_radius = args.value_of("context_radius").unwrap();
    let context_radius: usize = context_radius.parse().expect("context_radius parsing error");

    let text1 = read_file(file_1)?;
    let text2 = read_file(file_2)?;

    println!("--- {}\t{}", file_1, timestamp(file_1)?);
    println!("+++ {}\t{}", file_2, timestamp(file_2)?);

    for s in diff_rs::unidiff(&text1, &text2, context_radius)? {
        println!("{}", s);
    }
    Ok(())
}
