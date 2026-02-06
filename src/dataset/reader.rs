use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::Error;

pub struct FileReader(BufReader<File>);

impl FileReader {
    pub fn new(path: &Path) -> Result<Self, Error> {
        Ok(Self(BufReader::new(File::open(path)?)))
    }

    pub fn skip_lines(mut self, n: usize) -> Result<Self, Error> {
        let mut buf = vec![];
        for _ in 0..n {
            self.0.read_until(b'\n', &mut buf)?;
        }
        Ok(self)
    }

    pub fn read_line(&mut self, buf: &mut String) -> Result<usize, Error> {
        buf.clear();
        self.0.read_line(buf).map_err(Into::into)
    }
}
