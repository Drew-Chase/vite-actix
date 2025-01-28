use actix_web::error::ErrorInternalServerError;
use actix_web::{web, App, Error, HttpRequest, HttpResponse};
use awc::Client;
use futures_util::StreamExt;
use log::{debug, error};
// The maximum payload size allowed for forwarding requests and responses.
//
// This constant defines the maximum size (in bytes) for the request and response payloads
// when proxying. Any payload exceeding this size will result in an error.
//
// Currently, it is set to 1 GB.
const MAX_PAYLOAD_SIZE: usize = 1024 * 1024 * 1024; // 1 GB

// Proxies requests to the Vite development server.
//
// This function forwards incoming requests to a local Vite server running on port 3000.
// It buffers the entire request payload and response payload to avoid partial transfers.
// Requests and responses larger than the maximum payload size will result in an error.
//
// # Arguments
//
// * `req` - The HTTP request object.
// * `payload` - The request payload.
//
// # Returns
//
// An `HttpResponse` which contains the response from the Vite server,
// or an error response in case of failure.
async fn proxy_to_vite(
    req: HttpRequest,
    mut payload: web::Payload,
) -> anyhow::Result<HttpResponse, Error> {
    // Create a new HTTP client instance for making requests to the Vite server.
    let client = Client::new();

    // Construct the URL of the Vite server by reading the VITE_PORT environment variable,
    // defaulting to 5173 if the variable is not set.
    // The constructed URL uses the same URI as the incoming request.
    let forward_url = format!(
        "http://localhost:{}{}",
        std::env::var("VITE_PORT").unwrap_or("5173".to_string()),
        req.uri()
    );

    // Buffer the entire payload from the incoming request into body_bytes.
    // This accumulates all chunks of the request body until no more are received or
    // until the maximum allowed payload size is exceeded.
    let mut body_bytes = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // Check if the payload exceeds the maximum size defined by MAX_PAYLOAD_SIZE.
        if (body_bytes.len() + chunk.len()) > MAX_PAYLOAD_SIZE {
            return Err(actix_web::error::ErrorPayloadTooLarge("Payload overflow"));
        }
        // Append the current chunk to the body buffer.
        body_bytes.extend_from_slice(&chunk);
    }

    // Forward the request to the Vite server along with the buffered request body.
    let mut forwarded_resp = client
        .request_from(forward_url.as_str(), req.head()) // Clone headers and method from the original request.
        .no_decompress() // Disable automatic decompression of the response.
        .send_body(body_bytes) // Send the accumulated request payload to the Vite server.
        .await
        .map_err(|err| ErrorInternalServerError(format!("Failed to forward request: {}", err)))?;

    // Buffer the entire response body from the Vite server into resp_body_bytes.
    // This accumulates all chunks of the response body until no more are received or
    // until the maximum allowed payload size is exceeded.
    let mut resp_body_bytes = web::BytesMut::new();
    while let Some(chunk) = forwarded_resp.next().await {
        let chunk = chunk?;
        // Check if the response payload exceeds the maximum size defined by MAX_PAYLOAD_SIZE.
        if (resp_body_bytes.len() + chunk.len()) > MAX_PAYLOAD_SIZE {
            return Err(actix_web::error::ErrorPayloadTooLarge(
                "Response payload overflow",
            ));
        }
        // Append the current chunk to the response buffer.
        resp_body_bytes.extend_from_slice(&chunk);
    }

    // Build the HTTP response to send back to the client.
    let mut res = HttpResponse::build(forwarded_resp.status());

    // Copy all headers from the response received from the Vite server
    // and include them in the response to the client.
    for (header_name, header_value) in forwarded_resp.headers().iter() {
        res.insert_header((header_name.clone(), header_value.clone()));
    }

    // Return the response with the buffered body to the client.
    Ok(res.body(resp_body_bytes))
}

/// Starts a Vite server by locating the installation of the Vite command using the system's
/// `where` or `which` command (based on OS) and spawning the server in the configured working
/// directory.
///
/// # Returns
///
/// Returns a result containing the spawned process's [`std::process::Child`] handle if successful,
/// or an [`anyhow::Error`] if an error occurs.
///
/// # Errors
///
/// - Returns an error if the `vite` command cannot be found (`NotFound` error).
/// - Returns an error if the `vite` command fails to execute or produce valid output.
/// - Returns an error if the working directory environment variable or directory retrieval fails.
///
/// # Notes
///
/// - The working directory for Vite is set with the `VITE_WORKING_DIR` environment variable,
///   falling back to the result of `try_find_vite_dir` or the current directory (".").
///
/// # Example
/// ```no-rust
/// let server = start_vite_server().expect("Failed to start Vite server");
/// println!("Vite server started with PID: {}", server.id());
/// ```
///
/// # Platform-Specific
/// - On Windows, it uses `where` to find the `vite` executable.
/// - On other platforms, it uses `which`.
pub fn start_vite_server() -> anyhow::Result<std::process::Child> {
    #[cfg(target_os = "windows")]
    let find_cmd = "where"; // Use `where` on Windows to find the executable location.
    #[cfg(not(target_os = "windows"))]
    let find_cmd = "which"; // Use `which` on Unix-based systems to find the executable location.

    // Locate the `vite` executable by invoking the system command and checking its output.
    let vite = std::process::Command::new(find_cmd)
        .arg("vite")
        .stdout(std::process::Stdio::piped()) // Capture the command's stdout.
        .output()? // Execute the command and handle potential IO errors.
        .stdout;

    // Convert the command output from bytes to a UTF-8 string.
    let vite = String::from_utf8(vite)?;
    let vite = vite.as_str().trim(); // Trim whitespace around the command output.

    // If the `vite` command output is empty, the executable was not found.
    if vite.is_empty() {
        // Log an error message and return a `NotFound` error.
        error!("vite not found, make sure it's installed with npm install -g vite");
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "vite not found",
        ))?;
    }

    // Vite installation could have multiple paths; using the last occurrence is a safeguard.
    let vite = vite
        .split("\n") // Split the results line by line.
        .collect::<Vec<_>>() // Collect lines into a vector of strings.
        .last() // Take the last entry in the result list.
        .expect("Failed to get vite executable") // Panic if the vector for some reason is empty.
        .trim(); // Trim any extra whitespace around the final path.

    debug!("found vite at: {:?}", vite); // Log the found Vite path for debugging.

    // Set the working directory for the Vite server. Use the environment variable if set, or:
    // 1. Try to find the directory containing `vite.config.ts`.
    // 2. Fallback to the current directory ("./") if none is found.
    let working_dir =
        std::env::var("VITE_WORKING_DIR").unwrap_or(try_find_vite_dir().unwrap_or(".".to_string()));

    // Start the Vite server with the determined executable and working directory.
    Ok(
        std::process::Command::new(vite) // Start command using Vite executable.
            .current_dir(working_dir) // Set the working directory as determined above.
            .arg("--port")
            .arg(std::env::var("VITE_PORT").unwrap_or("5173".to_string()))
            .arg("-l")
            .arg("warn")
            .spawn()?, // Spawn the subprocess and propagate any errors.
    )
}

/// Attempts to find the directory containing `vite.config.ts`
/// by traversing the filesystem upwards from the current working directory.
///
/// # Returns
///
/// Returns `Some(String)` with the path of the directory containing the `vite.config.ts` file,
/// if found. Otherwise, returns `None` if the file is not located or an error occurs during traversal.
///
/// # Example
/// ```no-rust
/// if let Some(vite_dir) = try_find_vite_dir() {
///     println!("Found vite.config.ts in directory: {}", vite_dir);
/// } else {
///     println!("vite.config.ts not found.");
/// }
/// ```
pub fn try_find_vite_dir() -> Option<String> {
    // Get the current working directory. If unable to retrieve, return `None`.
    let mut cwd = std::env::current_dir().ok()?;

    // Continue traversing upwards in the directory hierarchy until the root directory is reached.
    while cwd != std::path::Path::new("/") {
        // Check if 'vite.config.ts' exists in the current directory.
        if cwd.join("vite.config.ts").exists() {
            // If found, convert the path to a `String` and return it.
            return Some(cwd.to_str()?.to_string());
        }
        // Move to the parent directory if it exists.
        if let Some(parent) = cwd.parent() {
            cwd = parent.to_path_buf();
        } else {
            // Break the loop if the parent directory doesn't exist.
            break;
        }
    }

    // Return `None` if 'vite.config.ts' was not found.
    None
}

/// Trait for configuring a Vite development proxy in an Actix web application.
///
/// This trait provides a method `configure_vite` to configure a web application
/// for proxying requests to the Vite development server during development,
/// while leaving the application unchanged in production.
pub trait ViteAppFactory {
    /// Configures the application to integrate with a Vite development proxy.
    ///
    /// This method configures the application to forward requests to a Vite
    /// development server, enabling features such as hot module replacement (HMR)
    /// during development. In a production environment, this configuration
    /// typically has no effect, ensuring no unnecessary overhead when serving
    /// static files or pre-compiled assets.
    ///
    /// # Returns
    ///
    /// Returns the modified application instance with the Vite proxy configuration applied.
    fn configure_vite(self) -> Self;
}

// Implementation of the `AppConfig` trait for Actix `App` instances.
impl<T> ViteAppFactory for App<T>
where
    T: actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest, // Type of the incoming HTTP request.
        Config = (),                    // No additional configuration is required.
        Error = Error,                  // Type of the error produced by the service.
        InitError = (),                 // No initialization error is expected.
    >,
{
    fn configure_vite(self) -> Self {
        if cfg!(debug_assertions) {
            // Add a default service to catch all unmatched routes and proxy them to Vite.
            self.default_service(web::route().to(proxy_to_vite))
                // Route requests for static assets to the Vite server (e.g., "/assets/<file>").
                .service(web::resource("/assets/{file:.*}").route(web::get().to(proxy_to_vite)))
                // Route requests for Node modules to the Vite server (e.g., "/node_modules/<file>").
                .service(
                    web::resource("/node_modules/{file:.*}").route(web::get().to(proxy_to_vite)),
                )
        } else {
            // If not in development mode, return the application without any additional configuration.
            self
        }
    }
}
