use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddr;

// Définir des tokens uniques pour chaque serveur
const SERVER1_8080: Token = Token(0);
const SERVER1_8081: Token = Token(1);
const SERVER2_8080: Token = Token(2);
const SERVER2_8081: Token = Token(3);

fn main() -> std::io::Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1024);

    // Configuration des serveurs
    let server1_addr1: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let server1_addr2: SocketAddr = "127.0.0.1:8081".parse().unwrap();
    let server2_addr1: SocketAddr = "127.0.0.2:8080".parse().unwrap();
    let server2_addr2: SocketAddr = "127.0.0.2:8081".parse().unwrap();

    let mut server1_8080 = TcpListener::bind(server1_addr1)?;
    let mut server1_8081 = TcpListener::bind(server1_addr2)?;
    let mut server2_8080 = TcpListener::bind(server2_addr1)?;
    let mut server2_8081 = TcpListener::bind(server2_addr2)?;

    poll.registry()
        .register(&mut server1_8080, SERVER1_8080, Interest::READABLE)?;
    poll.registry()
        .register(&mut server1_8081, SERVER1_8081, Interest::READABLE)?;
    poll.registry()
        .register(&mut server2_8080, SERVER2_8080, Interest::READABLE)?;
    poll.registry()
        .register(&mut server2_8081, SERVER2_8081, Interest::READABLE)?;

    let mut connections = HashMap::new();
    let mut unique_token = Token(4);

    println!("Serveurs HTTP démarrés sur:");
    println!("  - http://{}", server1_addr1);
    println!("  - http://{}", server1_addr2);
    println!("  - http://{}", server2_addr1);
    println!("  - http://{}", server2_addr2);

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER1_8080 => handle_connection(
                    &mut poll,
                    &server1_8080,
                    &mut connections,
                    &mut unique_token,
                )?,
                SERVER1_8081 => handle_connection(
                    &mut poll,
                    &server1_8081,
                    &mut connections,
                    &mut unique_token,
                )?,
                SERVER2_8080 => handle_connection(
                    &mut poll,
                    &server2_8080,
                    &mut connections,
                    &mut unique_token,
                )?,
                SERVER2_8081 => handle_connection(
                    &mut poll,
                    &server2_8081,
                    &mut connections,
                    &mut unique_token,
                )?,
                token => {
                    if let Some(conn) = connections.get_mut(&token) {
                        if event.is_readable() {
                            let mut buffer = [0; 1024];
                            match conn.read(&mut buffer) {
                                Ok(0) => {
                                    connections.remove(&token);
                                }
                                Ok(_) => {
                                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Hello, World!</h1></body></html>";
                                    conn.write_all(response.as_bytes())?;
                                    connections.remove(&token);
                                }
                                Err(e) => eprintln!("Erreur de lecture: {}", e),
                            }
                        }
                    }
                }
            }
        }
    }
}

fn handle_connection(
    poll: &mut Poll,
    server: &TcpListener,
    connections: &mut HashMap<Token, TcpStream>,
    unique_token: &mut Token,
) -> std::io::Result<()> {
    loop {
        match server.accept() {
            Ok((mut connection, addr)) => {
                println!("Nouvelle connexion HTTP: {}", addr);
                let token = *unique_token;
                poll.registry()
                    .register(&mut connection, token, Interest::READABLE)?;
                connections.insert(token, connection);
                *unique_token = Token(unique_token.0 + 1);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
