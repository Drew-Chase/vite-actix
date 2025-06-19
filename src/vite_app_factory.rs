use crate::proxy_to_vite;
use actix_web::{web, App, Error};

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
                .service(web::resource("/{file:.*}").route(web::get().to(proxy_to_vite)))
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
impl<T> ViteAppFactory for actix_web::Scope<T>
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
            self.default_service(web::route().to(proxy_to_vite))
                .service(web::resource("/{file:.*}").route(web::get().to(proxy_to_vite)))
                .service(
                    web::resource("/node_modules/{file:.*}").route(web::get().to(proxy_to_vite)),
                )
        } else {
            self
        }
    }
}
