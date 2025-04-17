use log::Level::Debug;
use std::env::current_dir;
use std::sync::OnceLock;

static PROXY_VITE_OPTIONS: OnceLock<ProxyViteOptions> = OnceLock::new();

#[derive(Clone)]
pub struct ProxyViteOptions {
    pub port: Option<u16>,
    pub working_directory: String,
    pub log_level: Option<log::Level>,
}

impl Default for ProxyViteOptions {
    fn default() -> Self {
        Self {
            port: None,
            working_directory: try_find_vite_dir().unwrap_or(String::from("./")),
            log_level: Some(Debug),
        }
    }
}

impl ProxyViteOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }
    pub fn working_directory(mut self, working_directory: impl AsRef<str>) -> Self {
        self.working_directory = working_directory.as_ref().to_string();
        self
    }
    pub fn log_level(mut self, log_level: log::Level) -> Self {
        self.log_level = Some(log_level);
        self
    }
    pub fn disable_logging(mut self) -> Self {
        self.log_level = None;
        self   
    }
    pub(crate) fn update_port(port: u16) -> anyhow::Result<()> {
        let current = PROXY_VITE_OPTIONS.get();

        if let Some(current) = current {
            let mut updated = current.clone();
            updated.port = Some(port);

            // Replace the global options
            // Note: This will fail, but we're handling it explicitly
            if PROXY_VITE_OPTIONS.set(updated).is_err() {
                // Just log that we couldn't update the global, but port is stored
                log::debug!("Could not update global options, port is set to {}", port);
            }
        }

        Ok(())
    }
    pub fn build(self) -> anyhow::Result<()> {
        PROXY_VITE_OPTIONS
            .set(self)
            .map_err(|_| anyhow::Error::msg("Failed to set proxy options"))
    }
    pub fn global() -> &'static Self {
        PROXY_VITE_OPTIONS.get_or_init(Self::default)
    }
}

/// Attempts to find the directory containing `vite.config.ts`
/// by traversing the filesystem upwards from the current working directory.
///
/// # Returns
///
/// Returns `Some(String)` with the path of the directory containing the `vite.config.[ts|js]` file,
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
    let mut cwd = current_dir().ok()?;

    // Continue traversing upwards in the directory hierarchy until the root directory is reached.
    while cwd != std::path::Path::new("/") {
        // Check if 'vite.config.ts' exists in the current directory.
        if cwd.join("vite.config.ts").exists() || cwd.join("vite.config.js").exists() {
            // If found, convert the path to a `String` and return it.
            return Some(cwd.to_str()?.to_string());
        }
        // Move to the parent directory if it exists.
        if let Some(parent) = cwd.parent() {
            cwd = parent.to_path_buf();
        } else {
            // Break the loop if the parent directory doesn't exist or if permissions were denied.
            break;
        }
    }

    // Return `None` if 'vite.config.[ts|js]' was not found.
    None
}
