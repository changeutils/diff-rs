//!
//! The GNU unidiff Rust library.
//!

extern crate diffs;

use std::{collections::VecDeque, io};

pub fn unidiff(
    text1: &[String],
    text2: &[String],
    context_radius: usize,
) -> io::Result<Vec<String>> {
    let mut processor = Processor::new(&text1, &text2, context_radius);
    {
        let mut replace = diffs::Replace::new(&mut processor);
        let _ = diffs::myers::diff(&mut replace, &text1, &text2)?;
    }
    Ok(processor.result())
}

struct Processor<'a> {
    text1: &'a [String],
    text2: &'a [String],

    context_radius: usize,
    inserted: usize,
    removed: usize,

    context: Context,
    result: Vec<String>,
}

impl<'a> Processor<'a> {
    pub fn new(text1: &'a [String], text2: &'a [String], context_radius: usize) -> Self {
        Self {
            text1,
            text2,

            context_radius,
            inserted: 0,
            removed: 0,

            context: Context::new(),
            result: Vec::new(),
        }
    }

    pub fn result(self) -> Vec<String> {
        self.result
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
        let mut start = if let Some(start) = self.start {
            start
        } else {
            return Vec::new();
        };
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

    fn equal(&mut self, old: usize, _new: usize, len: usize) -> Result<(), Self::Error> {
        if self.context.start.is_none() {
            self.context.start = Some(old);
        }

        self.context.counter = 0;
        for i in old..old + len {
            if !self.context.changed {
                if self.context.counter < self.context_radius {
                    self.context.data.push_back(format!(" {}", self.text1[i]));
                    self.context.equaled += 1;
                    self.context.counter += 1;
                }
                if self.context.counter >= self.context_radius {
                    self.context.data.push_back(format!(" {}", self.text1[i]));
                    self.context.data.pop_front();
                    if let Some(ref mut start) = self.context.start {
                        *start += 1;
                    }
                    self.context.counter += 1;
                }
            }
            if self.context.changed {
                if self.context.counter < self.context_radius * 2 {
                    self.context.data.push_back(format!(" {}", self.text1[i]));
                    self.context.equaled += 1;
                    self.context.counter += 1;
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
                    context.start = Some(i - 1);

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
        if self.context.start.is_none() {
            self.context.start = Some(old);
        }

        for i in old..old + old_len {
            self.context.data.push_back(format!("-{}", self.text1[i]));
        }
        for i in new..new + new_len {
            self.context.data.push_back(format!("+{}", self.text2[i]));
        }
        self.context.changed = true;
        self.context.removed += old_len;
        self.context.inserted += new_len;

        Ok(())
    }

    fn insert(&mut self, old: usize, new: usize, new_len: usize) -> Result<(), Self::Error> {
        if self.context.start.is_none() {
            self.context.start = Some(old);
        }

        for i in new..new + new_len {
            self.context.data.push_back(format!("+{}", self.text2[i]));
        }
        self.context.changed = true;
        self.context.inserted += new_len;

        Ok(())
    }

    fn delete(&mut self, old: usize, len: usize) -> Result<(), Self::Error> {
        if self.context.start.is_none() {
            self.context.start = Some(old);
        }

        for i in old..old + len {
            self.context.data.push_back(format!("-{}", self.text1[i]));
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
