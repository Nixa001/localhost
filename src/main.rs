mod utils;
mod server;

use server::config::{DEFAULT_PAGE, ERROR_403_PAGE, ERROR_404_PAGE, ERROR_500_PAGE, TIMEOUT};
use server::multiserver::MultiServer;
use server::config::get_requested_path;
use utils::file_ops::{generate_response, read_file};

fn main() -> std::io::Result<()> {
    let addrs = ["127.0.0.1:8080", "127.0.0.1:8081"];
    let mut multi_server = MultiServer::new(&addrs)?;
    multi_server.run()
}
