use actix_web::{web, App, HttpResponse, HttpServer};
use anyhow::Result;
use vite_actix::{set_vite_port, set_vite_working_dir, start_vite_server, ViteAppFactory};

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::builder().filter_level(log::LevelFilter::Debug).init();
    // Debug configuration: Only execute the following block in debug mode.
    if cfg!(debug_assertions) {
        // Set the working directory for Vite (change this to point to the directory with vite.config.(js|ts)).
        // The library will try to automatically detect this if not explicitly set.
        set_vite_working_dir("./examples/wwwroot");

        // Set the port Vite should use. By default, Vite uses port 5173.
        // Changing this allows running your application with a custom Vite server port.
        set_vite_port(3000);
    }

    // Debug configuration: Start the Vite development server only in debug mode.
    if cfg!(debug_assertions) {
        // Attempt to start the Vite server.
        // The function will locate and execute the Vite executable, logging any errors if it fails.
        start_vite_server().expect("Failed to start vite server");
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
