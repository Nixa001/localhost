mod server;
mod utils;

// use server::config::get_requested_path;
use server::multiserver::MultiServer;
// use server::config::{DEFAULT_PAGE, ERROR_403_PAGE, ERROR_404_PAGE, ERROR_500_PAGE, TIMEOUT};
// use utils::file_ops::{generate_response, read_file};

fn main() -> std::io::Result<()> {
    let configs = [
        (
            "0.0.0.0:8080",
            vec![
                ("server1.com", "src/www"),
                ("localhost", "src/www"),
                ("errors.com", "src/www/errors"),
            ],
        ),
        (
            "0.0.0.0:8081",
            vec![
                ("server2.com", "src/www/server"),
                ("localhost", "src/www/server"),
                ("localhost", "src/www/server"),
            ],
        ),
    ];
    let mut multi_server = MultiServer::new(&configs)?;
    multi_server.run()
}
