#![doc = include_str!("../README.md")]

pub mod proxy_vite_options;
pub mod vite_app_factory;

use crate::proxy_vite_options::ProxyViteOptions;
use actix_web::error::ErrorInternalServerError;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use awc::Client;
use futures_util::StreamExt;
use log::{debug, error, info, trace, warn};
use regex::Regex;

// The maximum payload size allowed for forwarding requests and responses.
//
// This constant defines the maximum size (in bytes) for the request and response payloads
// when proxying. Any payload exceeding this size will result in an error.
//
// Currently, it is set to 1 GB.
const MAX_PAYLOAD_SIZE: usize = 1024 * 1024 * 1024; // 1 GB

// Proxy requests to the Vite development server.
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
    
    // Get a copy of the current global options
    let options = ProxyViteOptions::global();
    
    let port = if let Some(port) = options.port {
        port
    } else {
        return Err(ErrorInternalServerError(
            "Unable to get port, you may have to set the port manually",
        ));
    };

    // Construct the URL of the Vite server by reading the VITE_PORT environment variable,
    // defaulting to 5173 if the variable is not set.
    // The constructed URL uses the same URI as the incoming request.
    let forward_url = format!("http://localhost:{}{}", port, req.uri());

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
///
/// # Clippy:
/// You may want to allow zombie processes in your code.   
/// `#[allow(clippy::zombie_processes)]`
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
        .split("\n") // Split the result line by line.
        .collect::<Vec<_>>() // Collect lines into a vector of strings.
        .last() // Take the last entry in the result list.
        .expect("Failed to get vite executable") // Panic if the vector for some reason is empty.
        .trim(); // Trim any extra whitespace around the final path.

    debug!("found vite at: {:?}", vite); // Log the found Vite path for debugging.

    let options = ProxyViteOptions::global();

    let mut vite_process = std::process::Command::new(vite);
    vite_process.current_dir(&options.working_directory);
    vite_process.stdout(std::process::Stdio::piped());

    if let Some(port) = options.port {
        vite_process.arg("--port").arg(port.to_string());
        //        vite_process.arg("--strictPort");
    }

    let mut vite_process = vite_process.spawn()?;

    // Create a buffered reader to capture the output from the Vite process.
    let vite_stdout = vite_process
        .stdout
        .take()
        .ok_or_else(|| anyhow::Error::msg("Failed to capture Vite process stdout"))?;

    // Clone options for the thread
    let options_clone = options.clone();

    // Create a channel to signal when Vite is ready
    let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);

    // Spawn a thread to handle stdout reading
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(vite_stdout);
        let mut line = String::new();

        // Create a Tokio runtime for this thread to handle async operations
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        let regex = Regex::new(r"(?P<url>http://localhost:\d+).*").unwrap();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // End of file reached, the process has likely terminated
                    debug!("End of output stream from Vite process, exiting reader loop");
                    break;
                }
                Ok(_) => {
                    let trimmed_line = line.trim().to_string();

                    // Send the line through the channel
                    // This will block until the message is sent,
                    // but that's okay because we're in a dedicated thread
                    if rt.block_on(tx.send(trimmed_line.clone())).is_err() {
                        debug!("Failed to send log line, receiver was dropped");
                        break;
                    }
                    let decolored_text =
                        String::from_utf8(strip_ansi_escapes::strip(trimmed_line.as_str()))
                            .unwrap();
                    if decolored_text.contains("Local")
                        && decolored_text.contains("http://localhost:")
                    {
                        let caps = regex.captures(&decolored_text).unwrap();
                        let url = caps.name("url").unwrap().as_str();
                        let port = url.split(":").last().unwrap();
                        let port: u16 = port.parse().unwrap();
                        
                        if let Err(e) = ProxyViteOptions::update_port(port) {
                            debug!("Failed to update Vite port to {}: {}", port, e);
                        } else {
                            debug!("Successfully updated Vite port to {}", port);
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to read line from Vite process: {}", err);
                    break;
                }
            }
        }
        debug!("Exiting Vite stdout reader thread");
    });

    // Spawn a task to receive messages and log them
    // This will work if we're in an async context with a Tokio runtime
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let options = options_clone.clone();
        handle.spawn(async move {
            let mut rx = rx;
            while let Some(line) = rx.recv().await {
                match options.log_level {
                    None => {}
                    Some(log::Level::Trace) => trace!("{}", line),
                    Some(log::Level::Debug) => debug!("{}", line),
                    Some(log::Level::Info) => info!("{}", line),
                    Some(log::Level::Warn) => warn!("{}", line),
                    Some(log::Level::Error) => error!("{}", line),
                }
            }
        });
    } else {
        // If we're not in a Tokio runtime context, we can create a thread to handle it
        std::thread::spawn(move || {
            // Create a runtime for this thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");

            rt.block_on(async move {
                let mut rx = rx;
                while let Some(line) = rx.recv().await {
                    match options_clone.log_level {
                        None => {}
                        Some(log::Level::Trace) => trace!("{}", line),
                        Some(log::Level::Debug) => debug!("{}", line),
                        Some(log::Level::Info) => info!("{}", line),
                        Some(log::Level::Warn) => warn!("{}", line),
                        Some(log::Level::Error) => error!("{}", line),
                    }
                }
            });
        });
    }

    // Return the process, which will continue running and logging output
    Ok(vite_process)
}
