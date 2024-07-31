use crate::config::ServerConfig;
use crate::http::handle_request;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Server {
    poll: Poll,
    events: Events,
    servers: Vec<(TcpListener, usize, u16)>,
    connections: HashMap<Token, Connection>,
    next_token: usize,
}

struct Connection {
    stream: TcpStream,
    server_id: usize,
    port: u16,
    last_activity: Instant,
}

impl Server {
    pub fn new(config: &ServerConfig) -> std::io::Result<Self> {
        let poll = Poll::new()?;
        let mut servers = Vec::new();
        let mut next_token = 0;

        for (index, addr) in config.addresses.iter().enumerate() {
            let mut server = TcpListener::bind(*addr)?;
            let port = addr.port();
            let token = Token(next_token);
            poll.registry()
                .register(&mut server, token, Interest::READABLE)?;
            servers.push((server, index + 1, port)); // server_id starts from 1
            next_token += 1;
        }

        Ok(Server {
            poll,
            events: Events::with_capacity(1024),
            servers,
            connections: HashMap::new(),
            next_token,
        })
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        loop {
            self.poll
                .poll(&mut self.events, Some(Duration::from_millis(100)))?;

            let mut actions = Vec::new();
            for event in self.events.iter() {
                let token = event.token();
                if token.0 < self.servers.len() {
                    actions.push(Action::Accept(token));
                } else if event.is_readable() {
                    actions.push(Action::Read(token));
                } else if event.is_writable() {
                    actions.push(Action::Write(token));
                }
            }

            for action in actions {
                match action {
                    Action::Accept(token) => self.accept_connection(token)?,
                    Action::Read(token) => self.handle_connection(token)?,
                    Action::Write(token) => self.write_response(token)?,
                }
            }

            self.clean_idle_connections();
        }
    }

    fn accept_connection(&mut self, server_token: Token) -> std::io::Result<()> {
        if let Some((listener, server_id, port)) = self.servers.get_mut(server_token.0) {
            let (mut stream, _) = listener.accept()?;
            let token = Token(self.next_token);
            self.poll
                .registry()
                .register(&mut stream, token, Interest::READABLE)?;

            self.connections.insert(
                token,
                Connection {
                    stream,
                    server_id: *server_id,
                    port: *port,
                    last_activity: Instant::now(),
                },
            );

            self.next_token += 1;
        }
        Ok(())
    }

    fn handle_connection(&mut self, token: Token) -> std::io::Result<()> {
        let mut buffer = [0; 1024];
        let mut remove = false;

        if let Some(conn) = self.connections.get_mut(&token) {
            match conn.stream.read(&mut buffer) {
                Ok(0) => remove = true,
                Ok(_) => {
                    conn.last_activity = Instant::now();
                    let response = handle_request(&buffer, conn.server_id, conn.port);
                    conn.stream.write_all(response.to_string().as_bytes())?;
                    self.poll
                        .registry()
                        .reregister(&mut conn.stream, token, Interest::WRITABLE)?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    eprintln!("Erreur de lecture: {}", e);
                    remove = true;
                }
            }
        }

        if remove {
            if let Some(mut conn) = self.connections.remove(&token) {
                self.poll.registry().deregister(&mut conn.stream)?;
            }
        }

        Ok(())
    }

    fn write_response(&mut self, token: Token) -> std::io::Result<()> {
        if let Some(conn) = self.connections.get_mut(&token) {
            self.poll
                .registry()
                .reregister(&mut conn.stream, token, Interest::READABLE)?;
        }
        Ok(())
    }

    fn clean_idle_connections(&mut self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();
        for (token, conn) in &mut self.connections {
            if now.duration_since(conn.last_activity) > KEEP_ALIVE_TIMEOUT {
                to_remove.push(*token);
            }
        }
        for token in to_remove {
            if let Some(mut conn) = self.connections.remove(&token) {
                let _ = self.poll.registry().deregister(&mut conn.stream);
            }
        }
    }
}

enum Action {
    Accept(Token),
    Read(Token),
    Write(Token),
}
