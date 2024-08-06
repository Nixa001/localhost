
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

pub struct StaticFile {
    pub content: Vec<u8>,
    pub file: File,
    pub size: u64,
    pub content_type: String,
    pub current_position: u64,
}

impl StaticFile {
    pub fn new(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        let metadata = file.metadata()?;
        let size = metadata.len();
        let content_type = mime_guess::from_path(path).first_or_octet_stream().to_string();

        Ok(StaticFile {
            content,
            file,
            size,
            content_type,
            current_position: 0,
        })
    }

    pub fn read_chunk(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.file.seek(SeekFrom::Start(self.current_position))?;
        let bytes_read = self.file.read(buffer)?;
        self.current_position += bytes_read as u64;
        Ok(bytes_read)
    }
}