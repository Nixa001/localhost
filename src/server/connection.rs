use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::time::Instant;

pub struct Connection {
    pub stream: TcpStream,
    pub buffer: Vec<u8>,
    pub response: Option<Vec<u8>>,
    pub last_activity: Instant,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Connection {
            stream,
            buffer: Vec::new(),
            response: None,
            last_activity: Instant::now(),
        }
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buffer)
    }

    pub fn write(&mut self, response: &[u8]) -> std::io::Result<()> {
        self.stream.write_all(response)
    }
}
