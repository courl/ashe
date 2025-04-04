use std::ops;
use std::path::Path;

pub struct Buffer {
    data: Vec<u8>,
    dirty: bool,
}

impl Buffer {
    pub fn new(file: &Path) -> Result<Self, std::io::Error> {
        Ok(Buffer {
            data: std::fs::read(file)?,
            dirty: false,
        })
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn update(&mut self, index: usize, data: u8) {
        self.dirty = true;
        self.data[index] = data;
    }

    pub fn save(&mut self, path: Box<Path>) -> Result<(), std::io::Error> {
        match std::fs::write(path, &self.data) {
            Ok(_) => {
                self.dirty = false;
                Ok(())
            }
            error => error,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl ops::Index<usize> for Buffer {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}
