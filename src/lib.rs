//!
//! The GNU unidiff Rust library.
//!

extern crate diffs;
#[macro_use]
extern crate log;
extern crate chrono;

use std::{
    collections::VecDeque,
    fs,
    io::{self, BufRead},
};

use chrono::{DateTime, Local};

fn read_file(path: &str) -> io::Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let file = io::BufReader::new(file);
    file.lines().collect()
}

fn header(path: &str) -> io::Result<String> {
    let metadata = fs::metadata(path)?;
    let filetime: DateTime<Local> = DateTime::from(metadata.modified()?);
    Ok(format!(
        "--- {}\t{}",
        path,
        filetime.format("%Y-%m-%d %H:%M:%S.%f %z").to_string(),
    ))
}

pub fn unidiff(
    file1_path: &str,
    file2_path: &str,
    context_radius: usize,
) -> io::Result<Vec<String>> {
    let file1 = read_file(file1_path)?;
    let file2 = read_file(file2_path)?;

    let mut processor = Processor::new(&file1, &file2, context_radius);
    {
        let mut replace = diffs::Replace::new(&mut processor);
        let _ = diffs::myers::diff(&mut replace, &file1, &file2)?;
    }

    let mut data = Vec::with_capacity(2);
    data.push(header(file1_path)?);
    data.push(header(file2_path)?);

    let unidiff = processor.result();
    if unidiff.len() > 0 {
        data.append(unidiff);
        Ok(data)
    } else {
        Ok(Vec::new())
    }
}

pub struct Processor<'a> {
    file1: &'a [String],
    file2: &'a [String],

    context_radius: usize,
    inserted: usize,
    removed: usize,

    context: Context,
    result: Vec<String>,
}

impl<'a> Processor<'a> {
    pub fn new(file1: &'a [String], file2: &'a [String], context_radius: usize) -> Self {
        Self {
            file1,
            file2,

            context_radius,
            inserted: 0,
            removed: 0,

            context: Context::new(),
            result: Vec::new(),
        }
    }

    pub fn result(&mut self) -> &mut Vec<String> {
        &mut self.result
    }
}

struct Context {
    pub start: Option<usize>,
    pub data: VecDeque<String>,
    pub changed: bool,

    pub counter: usize,
    pub equaled: usize,
    pub removed: usize,
    pub inserted: usize,
}

impl Context {
    pub fn new() -> Self {
        Self {
            start: None,
            data: VecDeque::new(),
            changed: false,

            counter: 0,
            equaled: 0,
            removed: 0,
            inserted: 0,
        }
    }

    pub fn to_vec(&self, removed: usize, inserted: usize) -> Vec<String> {
        let start = if let Some(start) = self.start {
            start + 1
        } else {
            return Vec::new();
        };
        let mut data = Vec::with_capacity(self.data.len() + 1);
        if self.changed {
            data.push(format!(
                "@@ -{},{} +{},{} @@",
                start,
                self.equaled + self.removed,
                start + inserted - removed,
                self.equaled + self.inserted,
            ));
            for s in self.data.iter() {
                data.push(format!("{}", s));
            }
        }
        data
    }
}

impl<'a> diffs::Diff for Processor<'a> {
    type Error = io::Error;

    fn equal(&mut self, old: usize, new: usize, len: usize) -> Result<(), Self::Error> {
        debug!("EQUAL {} {} {}", old, new, len);
        if self.context.start.is_none() {
            self.context.start = Some(old);
        }

        self.context.counter = 0;
        for i in old..old + len {
            if !self.context.changed {
                if self.context.counter < self.context_radius {
                    self.context.data.push_back(format!(" {}", self.file1[i]));
                    self.context.equaled += 1;
                    self.context.counter += 1;
                    debug!(
                        "NOT CHANGED YET. PUSHED (counter = {})",
                        self.context.counter
                    );
                }
                if self.context.counter >= self.context_radius {
                    self.context.data.push_back(format!(" {}", self.file1[i]));
                    self.context.data.pop_front();
                    if let Some(ref mut start) = self.context.start {
                        *start += 1;
                    }
                    self.context.counter += 1;
                    debug!(
                        "NOT CHANGED YET. PUSHED AND POPPED (counter = {})",
                        self.context.counter
                    );
                }
            }
            if self.context.changed {
                if self.context.counter < self.context_radius * 2 {
                    self.context.data.push_back(format!(" {}", self.file1[i]));
                    self.context.equaled += 1;
                    self.context.counter += 1;
                    debug!(
                        "CHANGED ALREADY. PUSHED (counter = {})",
                        self.context.counter
                    );
                }
                if self.context.counter == self.context_radius && len > self.context_radius * 2 {
                    self.result
                        .append(&mut self.context.to_vec(self.removed, self.inserted));

                    let mut context = Context::new();
                    for _ in 0..self.context_radius {
                        context.data.push_back(String::new());
                    }
                    context.counter = self.context_radius;
                    context.equaled = self.context_radius;
                    context.start = Some(i - 2);

                    self.removed += self.context.removed;
                    self.inserted += self.context.inserted;
                    self.context = context;
                }
            }
        }

        Ok(())
    }

    fn replace(
        &mut self,
        old: usize,
        old_len: usize,
        new: usize,
        new_len: usize,
    ) -> Result<(), Self::Error> {
        debug!("REPLACE {} {} {} {}", old, old_len, new, new_len);
        if self.context.start.is_none() {
            self.context.start = Some(old);
        }

        for i in old..old + old_len {
            self.context.data.push_back(format!("-{}", self.file1[i]));
        }
        for i in new..new + new_len {
            self.context.data.push_back(format!("+{}", self.file2[i]));
        }
        self.context.changed = true;
        self.context.removed += old_len;
        self.context.inserted += new_len;

        Ok(())
    }

    fn insert(&mut self, old: usize, new: usize, new_len: usize) -> Result<(), Self::Error> {
        debug!("INSERT {} {} {}", old, new, new_len);
        if self.context.start.is_none() {
            self.context.start = Some(old);
        }

        for i in new..new + new_len {
            self.context.data.push_back(format!("+{}", self.file2[i]));
        }
        self.context.changed = true;
        self.context.inserted += new_len;

        Ok(())
    }

    fn delete(&mut self, old: usize, len: usize) -> Result<(), Self::Error> {
        debug!("DELETE {} {}", old, len);
        if self.context.start.is_none() {
            self.context.start = Some(old);
        }

        for i in old..old + len {
            self.context.data.push_back(format!("-{}", self.file1[i]));
        }
        self.context.changed = true;
        self.context.removed += len;

        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        if self.context.counter > self.context_radius {
            let truncation = self.context.counter - self.context_radius;
            if self.context.data.len() > truncation {
                let new_size = self.context.data.len() - truncation;
                self.context.equaled -= truncation;
                self.context.data.truncate(new_size);
            }
        }
        self.result
            .append(&mut self.context.to_vec(self.removed, self.inserted));
        Ok(())
    }
}
