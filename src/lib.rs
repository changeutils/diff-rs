//!
//! The GNU unidiff Rust library.
//!

extern crate diffs;
#[macro_use]
extern crate log;
extern crate chrono;

use std::{
    io::{self, BufRead},
    fs,
    collections::VecDeque,
};

use chrono::{DateTime, Local};

fn read_file(path: &str) -> io::Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let file = io::BufReader::new(file);
    let mut err = None;
    let file = file
        .lines()
        .map(|result| match result {
            Ok(data) => data,
            Err(error) => {
                err = Some(error);
                String::new()
            }
        })
        .collect::<Vec<String>>();
    if let Some(err) = err {
        return Err(err);
    }
    Ok(file)
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

pub fn unidiff(file1_path: &str, file2_path: &str, context_radius: usize) -> io::Result<Vec<String>> {
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

    changeset: ChangeSet,
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

            changeset: ChangeSet::new(),
            result: Vec::new(),
        }
    }

    pub fn result(&mut self) -> &mut Vec<String> {
        &mut self.result
    }
}

struct ChangeSet {
    pub start: Option<usize>,
    pub data: VecDeque<String>,
    pub counter: usize,

    pub changed: bool,

    pub equaled: usize,
    pub removed: usize,
    pub inserted: usize,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self {
            start: None,
            data: VecDeque::new(),
            counter: 0,

            changed: false,

            equaled: 0,
            removed: 0,
            inserted: 0,
        }
    }

    pub fn to_vec(&self, removed: usize, inserted: usize) -> Vec<String> {
        let mut start = self.start.unwrap();
        if start == 0 {
            start = 1;
        }
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
        if self.changeset.start.is_none() {
            self.changeset.start = Some(old);
            debug!("START INIT {}", self.changeset.start.unwrap());
        }

        self.changeset.counter = 0;
        for i in old..old+len {
            if !self.changeset.changed {
                if self.changeset.counter < self.context_radius {
                    self.changeset.data.push_back(format!(" {}", self.file1[i]));
                    self.changeset.equaled += 1;
                    self.changeset.counter += 1;
                    debug!("NOT CHANGED YET. PUSHED (counter = {})", self.changeset.counter);
                }
                if self.changeset.counter >= self.context_radius {
                    self.changeset.data.push_back(format!(" {}", self.file1[i]));
                    self.changeset.data.pop_front();
                    if let Some(ref mut start) = self.changeset.start {
                        *start += 1;
                        debug!("START EDIT {}", start);
                    }
                    self.changeset.counter += 1;
                    debug!("NOT CHANGED YET. PUSHED AND POPPED (counter = {})", self.changeset.counter);
                }
            }
            if self.changeset.changed {
                if self.changeset.counter < self.context_radius * 2 {
                    self.changeset.data.push_back(format!(" {}", self.file1[i]));
                    self.changeset.equaled += 1;
                    self.changeset.counter += 1;
                    debug!("CHANGED ALREADY. PUSHED (counter = {})", self.changeset.counter);
                }
                if self.changeset.counter == self.context_radius && len > self.context_radius * 2 {
                    self.result.append(&mut self.changeset.to_vec(self.removed, self.inserted));

                    let mut changeset = ChangeSet::new();
                    changeset.data.push_back("".to_owned());
                    changeset.data.push_back("".to_owned());
                    changeset.data.push_back("".to_owned());
                    changeset.counter = self.context_radius;
                    changeset.equaled = self.context_radius;
                    changeset.start = Some(i-1);

                    self.removed += self.changeset.removed;
                    self.inserted += self.changeset.inserted;
                    self.changeset = changeset;               
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
        if self.changeset.start.is_none() {
            self.changeset.start = Some(old);
            debug!("START INIT {}", self.changeset.start.unwrap());
        }
        
        for i in old..old+old_len {
            self.changeset.data.push_back(format!("-{}", self.file1[i]));
        }
        for i in new..new+new_len {
            self.changeset.data.push_back(format!("+{}", self.file2[i]));
        }
        self.changeset.changed = true;
        self.changeset.removed += old_len;
        self.changeset.inserted += new_len;

        Ok(())
    }

    fn insert(&mut self, old: usize, new: usize, new_len: usize) -> Result<(), Self::Error> {
        debug!("INSERT {} {} {}", old, new, new_len);
        if self.changeset.start.is_none() {
            self.changeset.start = Some(old);
            debug!("START INIT {}", self.changeset.start.unwrap());
        }
        
        for i in new..new + new_len {
            self.changeset.data.push_back(format!("+{}", self.file2[i]));
        }
        self.changeset.changed = true;
        self.changeset.inserted += new_len;

        Ok(())
    }

    fn delete(&mut self, old: usize, len: usize) -> Result<(), Self::Error> {
        debug!("DELETE {} {}", old, len);
        if self.changeset.start.is_none() {
            self.changeset.start = Some(old);
            debug!("START INIT {}", self.changeset.start.unwrap());
        }

        for i in old..old + len {
            self.changeset.data.push_back(format!("-{}", self.file1[i]));
        }
        self.changeset.changed = true;
        self.changeset.removed += len;

        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        if self.changeset.counter > self.context_radius {
            let truncation = self.changeset.counter - self.context_radius;
            if self.changeset.data.len() > truncation {
                let new_size = self.changeset.data.len() - truncation;
                self.changeset.equaled -= truncation;
                self.changeset.data.truncate(new_size);
            }
        }
        self.result.append(&mut self.changeset.to_vec(self.removed, self.inserted));
        Ok(())
    }
}
