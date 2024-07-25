mod server;
mod utils;

// use server::config::get_requested_path;
use server::multiserver::MultiServer;
// use server::config::{DEFAULT_PAGE, ERROR_403_PAGE, ERROR_404_PAGE, ERROR_500_PAGE, TIMEOUT};
// use utils::file_ops::{generate_response, read_file};

fn main() -> std::io::Result<()> {
    let configs = [
        (
            "127.0.0.1:8080",
            vec![
                ("test.com", "src/www/test"),
                ("example.com", "src/www/example"),
            ],
        ),
        ("127.0.0.1:8081", vec![("another.com", "src/www/another")]),
    ];
    let mut multi_server = MultiServer::new(&configs)?;
    multi_server.run()
}
