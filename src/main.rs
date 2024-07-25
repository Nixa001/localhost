use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

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
}

struct MultiServer {
    servers: Vec<Server>,
    poll: Poll,
    events: Events,
    next_token: usize,
}

impl Server {
    fn new(addr: &str) -> io::Result<Self> {
        let addr = addr.parse().unwrap();
        let listener = TcpListener::bind(addr)?;

        Ok(Server {
            listener,
            connections: HashMap::new(),
        })
    }
}

impl MultiServer {
    fn new(addrs: &[&str]) -> io::Result<Self> {
        let poll = Poll::new()?;
        let mut servers = Vec::new();

        for (idx, &addr) in addrs.iter().enumerate() {
            let mut server = Server::new(addr)?;
            poll.registry()
                .register(&mut server.listener, Token(idx), Interest::READABLE)?;
            servers.push(server);
        }

        Ok(MultiServer {
            servers,
            poll,
            events: Events::with_capacity(1024),
            next_token: addrs.len(),
        })
    }

    fn run(&mut self) -> io::Result<()> {
        loop {
            self.poll.poll(&mut self.events, Some(TIMEOUT))?;

            let mut events_to_handle = Vec::new();
            for event in self.events.iter() {
                events_to_handle.push(event.token());
            }

            for token in events_to_handle {
                if token.0 < self.servers.len() {
                    self.accept_new_connection(token.0)?;
                } else {
                    self.handle_client_event(token)?;
                }
            }

            self.check_timeouts()?;
        }
    }

    fn accept_new_connection(&mut self, server_idx: usize) -> io::Result<()> {
        let server = &mut self.servers[server_idx];
        loop {
            match server.listener.accept() {
                Ok((mut stream, _)) => {
                    let token = Token(self.next_token);
                    self.next_token += 1;
                    self.poll.registry().register(
                        &mut stream,
                        token,
                        Interest::READABLE | Interest::WRITABLE,
                    )?;
                    server.connections.insert(
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
        let server_idx = self.find_server_for_token(token);
        if let Some(server_idx) = server_idx {
            let server = &mut self.servers[server_idx];
            let mut remove_connection = false;
            let mut response_to_send = None;

            if let Some(conn) = server.connections.get_mut(&token) {
                let mut buffer = [0; 1024];
                match conn.stream.read(&mut buffer) {
                    Ok(0) => {
                        remove_connection = true;
                    }
                    Ok(n) => {
                        conn.buffer.extend_from_slice(&buffer[..n]);
                        conn.last_activity = Instant::now();
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
                if let Some(mut conn) = server.connections.remove(&token) {
                    self.poll.registry().deregister(&mut conn.stream)?;
                }
            }

            if let Some((token, response)) = response_to_send {
                if let Some(conn) = server.connections.get_mut(&token) {
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
        }

        Ok(())
    }

    fn find_server_for_token(&self, token: Token) -> Option<usize> {
        self.servers
            .iter()
            .position(|server| server.connections.contains_key(&token))
    }

    fn is_request_complete(buffer: &[u8]) -> bool {
        buffer.windows(4).any(|window| window == b"\r\n\r\n")
    }

    fn process_request(_buffer: &[u8]) -> Vec<u8> {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!";
        response.as_bytes().to_vec()
    }

    fn check_timeouts(&mut self) -> io::Result<()> {
        let now = Instant::now();
        for server in &mut self.servers {
            let timeout_tokens: Vec<Token> = server
                .connections
                .iter()
                .filter(|(_, conn)| now.duration_since(conn.last_activity) > TIMEOUT)
                .map(|(token, _)| *token)
                .collect();

            for token in timeout_tokens {
                if let Some(mut conn) = server.connections.remove(&token) {
                    self.poll.registry().deregister(&mut conn.stream)?;
                }
            }
        }

        Ok(())
    }
}

fn main() -> io::Result<()> {
    let addrs = ["127.0.0.1:8080", "127.0.0.1:8081"];
    let mut multi_server = MultiServer::new(&addrs)?;
    multi_server.run()
}
