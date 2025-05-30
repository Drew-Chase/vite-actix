# Vite Actix

![badge](https://github.com/Drew-Chase/vite-actix/actions/workflows/rust.yml/badge.svg)

Vite Actix is a library designed to enable seamless integration of the **Vite development server** with the **Actix web framework**. It provides proxying functionality to forward HTTP requests to a local Vite server during development, enabling support for features like **hot module replacement (HMR)**, while maintaining a production-ready design for serving static files.

---

## Features

- **Development Proxy**  
  Forwards unmatched HTTP requests to the Vite development server during development.

- **Hot Module Replacement**  
  Enables fast reloads of assets and code during development, boosting productivity.

- **Production-Ready**  
  Automatically serves pre-bundled assets in production without proxy overhead.

- **Customizable Configuration**  
  Supports environment variables for customizing Vite integration (e.g., working directory and port).

---

## Getting Started

### Prerequisites

Make sure you have the following tools installed:

- **Rust** (version 1.65 or higher recommended)
- **Node.js** (for Vite, version 18+ recommended)
- **npm/yarn/pnpm** (for managing front-end dependencies)

### Installation

Add the library to your Rust project by including it in your `Cargo.toml` file:

```toml
[dependencies]
vite-actix = "0.2.1"
```

or using git

```toml
[dependencies]
vite-actix = { git = "https://github.com/Drew-Chase/vite-actix.git" }
```

---

## Usage

### Basic Configuration and Setup

Follow these steps to integrate Vite with an Actix application:
1. **Example: Configuring Your Main Actix App**:
   Create a basic Actix application that includes Vite integration:

   ```rust
   use actix_web::{web, App, HttpResponse, HttpServer};
   use anyhow::Result;
   use vite_actix::{start_vite_server, ViteAppFactory, ProxyViteOptions};
   
   #[actix_web::main]
   async fn main() -> Result<()> {
       if cfg!(debug_assertions) {
            // Configure Vite options using the builder pattern
            ProxyViteOptions::new()
                .working_directory("./examples/wwwroot") // Directory containing vite.config.(js|ts)
                .port(3000) // Custom port for Vite (default is 5173)
                .build()?;
       }

       let server = HttpServer::new(move || {
           App::new()
               .route("/api/", web::get().to(HttpResponse::Ok))
               .configure_vite() // Enable Vite proxy during development
       })
       .bind("127.0.0.1:8080")?
       .run();

       if cfg!(debug_assertions) {
           start_vite_server()?;
       }

       println!("Server running at http://127.0.0.1:8080/");
       Ok(server.await?)
   }
   ```

3. **Run the Vite Dev Server**:
    - Use `vite-actix`'s `start_vite_server` function to automatically run the Vite server in development mode.
    - Static files and modules (such as `/assets/...`) are proxied to Vite when `cfg!(debug_assertions)` is true.

4. **Advanced Vite Configuration Options**:
   ```rust
   // Example of additional Vite configuration options
   if cfg!(debug_assertions) {
       ProxyViteOptions::new()
           .port(3000)                           // Custom Vite server port
           .working_directory("./frontend")      // Custom working directory
           .log_level(log::Level::Info)          // Configure log level
           // OR disable logging entirely
           // .disable_logging()
           .build()?;
   }
   ```

---

## Configuration

### Using ProxyViteOptions Builder

The recommended way to configure Vite integration is using the `ProxyViteOptions` builder pattern:

### Proxy Rules

The following routes are automatically proxied to the Vite dev server during development:

- **Default Service**: Proxies all unmatched routes.
- **Static Assets**: Requests for `/assets/...` are forwarded to the Vite server.
- **Node Modules**: Resolves `/node_modules/...` through Vite.

Ensure that your Vite configuration is consistent with the paths and routes used by your Actix web server.

---

## License

This project is licensed under the GNU General Public License v3.0.  
See the [LICENSE](./LICENSE) file for details.

---

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature-name`).
3. Commit your changes (`git commit -m "Description of changes"`).
4. Push to the branch (`git push origin feature-name`).
5. Open a pull request.

---

## Repository & Support

- **Repository**: [Vite Actix GitHub](https://github.com/Drew-Chase/vite-actix)
- **Issues**: Use the GitHub issue tracker for bug reports and feature requests.
- **Contact**: Reach out to the maintainer via the email listed in the repository.

---

## Examples

See the [`/examples`](https://github.com/Drew-Chase/vite-actix/tree/master/examples) directory for sample implementations, including a fully functional integration of Vite with an Actix service.

---

## Acknowledgements

- **Rust** for providing the ecosystem to build fast, secure web backends.
- **Vite** for its cutting-edge tooling in front-end development.

---

Enjoy using **Vite Actix** for your next project! If you encounter any issues, feel free to open a ticket on GitHub. 🛠️

