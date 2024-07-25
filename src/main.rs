use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

const SERVER: Token = Token(0);
const TIMEOUT: Duration = Duration::from_secs(30);

struct Connection {
    stream: TcpStream,
    buffer: Vec<u8>,
    response: Option<Vec<u8>>,
    last_activity: Instant,
}

struct Server {
    listener: TcpListener,
    connections: HashMap<Token, Connection>,
    poll: Poll,
    events: Events,
    next_token: usize,
}

impl Server {
    fn new(addr: &str) -> io::Result<Self> {
        let addr = addr.parse().unwrap();
        let mut listener = TcpListener::bind(addr)?;

        let poll = Poll::new()?;
        poll.registry()
            .register(&mut listener, SERVER, Interest::READABLE)?;

        Ok(Server {
            listener,
            connections: HashMap::new(),
            poll,
            events: Events::with_capacity(1024),
            next_token: 1,
        })
    }

    fn run(&mut self) -> io::Result<()> {
        loop {
            self.poll.poll(&mut self.events, Some(TIMEOUT))?;

            let mut tokens: Vec<Token> = Vec::new();

            for event in self.events.iter() {
                tokens.push(event.token());
            }

            for token in tokens {
                match token {
                    SERVER => self.accept_new_connections()?,
                    client_token => self.handle_client_event(client_token)?,
                }
            }

            self.check_timeouts()?;
        }
    }

    fn accept_new_connections(&mut self) -> io::Result<()> {
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    let token = Token(self.next_token);
                    self.next_token += 1;
                    self.poll.registry().register(
                        &mut stream,
                        token,
                        Interest::READABLE | Interest::WRITABLE,
                    )?;
                    self.connections.insert(
                        token,
                        Connection {
                            stream,
                            buffer: Vec::new(),
                            response: None,
                            last_activity: Instant::now(),
                        },
                    );
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn handle_client_event(&mut self, token: Token) -> io::Result<()> {
        let mut remove_connection = false;
        let mut response_to_send = None;

        if let Some(conn) = self.connections.get_mut(&token) {
            let mut buffer = [0; 1024];
            match conn.stream.read(&mut buffer) {
                Ok(0) => {
                    // Connection was closed
                    remove_connection = true;
                }
                Ok(n) => {
                    conn.buffer.extend_from_slice(&buffer[..n]);
                    conn.last_activity = Instant::now();
                    // Process the request if it's complete
                    if Self::is_request_complete(&conn.buffer) {
                        let response = Self::process_request(&conn.buffer);
                        conn.response = Some(response);
                        conn.buffer.clear();
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }

            if let Some(response) = conn.response.take() {
                response_to_send = Some((token, response));
            }
        }

        if remove_connection {
            self.connections.remove(&token);
        }

        if let Some((token, response)) = response_to_send {
            if let Some(conn) = self.connections.get_mut(&token) {
                match conn.stream.write(&response) {
                    Ok(n) if n < response.len() => {
                        conn.response = Some(response[n..].to_vec());
                    }
                    Ok(_) => {
                        conn.last_activity = Instant::now();
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        conn.response = Some(response);
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(())
    }

    fn is_request_complete(buffer: &[u8]) -> bool {
        // Implement logic to check if the HTTP request is complete
        // This is a simplified check, you might need more robust parsing
        buffer.windows(4).any(|window| window == b"\r\n\r\n")
    }

    fn process_request(buffer: &[u8]) -> Vec<u8> {
        // Implement request processing logic here
        // For now, we'll just return a simple "Hello, World!" response
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!";
        response.as_bytes().to_vec()
    }

    fn check_timeouts(&mut self) -> io::Result<()> {
        let now = Instant::now();
        let timeout_tokens: Vec<Token> = self
            .connections
            .iter()
            .filter(|(_, conn)| now.duration_since(conn.last_activity) > TIMEOUT)
            .map(|(token, _)| *token)
            .collect();

        for token in timeout_tokens {
            if let Some(mut conn) = self.connections.remove(&token) {
                self.poll.registry().deregister(&mut conn.stream)?;
            }
        }

        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut server = Server::new("127.0.0.1:8080")?;
    server.run()
}
