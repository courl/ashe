use std::ops;
use std::path::Path;

pub struct Buffer {
    data: Vec<u8>,
    dirty: bool,
}

impl Buffer {
    pub fn new(data: Vec<u8>) -> Self {
        Buffer { data, dirty: false }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn update(&mut self, index: usize, data: u8) {
        self.dirty = true;
        self.data[index] = data;
    }

    pub fn save(&mut self, path: &Path) -> Result<(), std::io::Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_new_buffer() {
        let data = vec![1, 2, 3];
        let buffer = Buffer::new(data.clone());

        assert_eq!(buffer.len(), data.len());
        assert_eq!(buffer.dirty, false);
        assert_eq!(buffer[0], 1);
        assert_eq!(buffer[1], 2);
        assert_eq!(buffer[2], 3);
    }

    #[test]
    fn test_buffer_len() {
        let buffer = Buffer::new(vec![1, 2, 3]);
        assert_eq!(buffer.len(), 3);

        let empty_buffer = Buffer::new(vec![]);
        assert_eq!(empty_buffer.len(), 0);
    }

    #[test]
    fn test_update() {
        let mut buffer = Buffer::new(vec![1, 2, 3]);
        buffer.update(1, 5);

        assert!(buffer.dirty);
        assert_eq!(buffer[1], 5);
    }

    #[test]
    fn test_is_dirty() {
        let mut buffer = Buffer::new(vec![1, 2, 3]);
        assert!(!buffer.is_dirty());

        buffer.update(1, 5);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn test_save_success() {
        let mut buffer = Buffer::new(vec![1, 2, 3]);
        let path = Path::new("test_save_success.bin");

        assert!(buffer.save(path).is_ok());
        assert!(!buffer.is_dirty());

        let saved_data = fs::read(path).unwrap();
        assert_eq!(saved_data, vec![1, 2, 3]);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_save_error() {
        let mut buffer = Buffer::new(vec![1, 2, 3]);
        buffer.update(1, 5);

        let path = Path::new("/invalid/test_save_error.bin");
        assert!(buffer.is_dirty());
        assert!(buffer.save(path).is_err());
        assert!(buffer.is_dirty());
    }

    #[test]
    fn test_index_access() {
        let buffer = Buffer::new(vec![1, 2, 3]);

        assert_eq!(buffer[0], 1);
        assert_eq!(buffer[1], 2);
        assert_eq!(buffer[2], 3);
    }
}
