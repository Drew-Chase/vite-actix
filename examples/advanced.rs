use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Result;
use log::{error, info};
use vite_actix::proxy_vite_options::ProxyViteOptions;
use vite_actix::start_vite_server;
use vite_actix::vite_app_factory::ViteAppFactory;

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_timestamp(None)
        .init();
    if cfg!(debug_assertions) {
        ProxyViteOptions::new()
            .port(3000)
            .working_directory("./")
            .disable_logging() // Disable logging from the Vite server.
            .log_level(log::Level::Warn) // Enables logging and sets the Vite server log level to "info".
            .build()?;

        std::thread::spawn(|| {
            loop {
                info!("Starting Vite server in development mode...");
                let status = start_vite_server()
                    .expect("Failed to start vite server")
                    .wait()
                    .expect("Vite server crashed!");
                if !status.success() {
                    error!("The vite server has crashed!");
                } else {
                    break;
                }
            }
        });
    }

    // Create the Actix web server instance.
    let server = HttpServer::new(move || {
        App::new()
            // Define an API route (e.g., "/api/") that returns an HTTP 200 OK response.
            .route("/api/", web::get().to(HttpResponse::Ok))
            // Configure the app to proxy requests to the Vite dev server.
            // This is primarily useful during development for features like hot module replacement (HMR).
            .configure_vite()
    })
    // Bind the Actix server to the address and port "127.0.0.1:8080".
    .bind("127.0.0.1:8080".to_string())?
    .run(); // Start the server asynchronously.

    // Output the server information, indicating where the application is accessible.
    println!("Server running at http://127.0.0.1:8080/");

    // Await the server's completion and propagate any errors that occur.
    Ok(server.await?)
}
