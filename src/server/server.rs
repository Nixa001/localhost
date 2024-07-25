use std::collections::HashMap;
use std::net::TcpListener;

use super::connection::Connection;
pub struct Server {
    pub listener: TcpListener,
    pub connections: HashMap<usize, Connection>,
    pub virtual_hosts: Vec<VirtualHost>,
}

pub struct VirtualHost {
    pub hostname: String,
    pub root_directory: String,
}
impl Server {
    pub fn new(addr: &str, virtual_hosts: Vec<VirtualHost>) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Server {
            listener,
            connections: HashMap::new(),
            virtual_hosts,
        })
    }
}
