use core::time::Duration;

use app::{App, shell};
use app_domain::AppState;
use axum::extract::FromRef;
use axum::routing::get;
use axum::{Router, http};
use http::{HeaderName, HeaderValue};
use leptos::config::LeptosOptions;
use leptos::prelude::provide_context;
use leptos_axum::{AxumRouteListing, LeptosRoutes, generate_route_list};
use tower::ServiceBuilder;
use tower_http::LatencyUnit;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

/// Largest request body accepted.
///
/// The forms in this application post a few hundred bytes. Without a ceiling a
/// single client can stream an unbounded body and exhaust memory.
const MAX_BODY_BYTES: usize = 256 * 1024;

/// Longest a request may run before the connection is released.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Wasm needs `wasm-unsafe-eval`; it permits WebAssembly compilation and
/// nothing else, unlike the far broader `unsafe-eval`.
///
/// `script-src` also carries `unsafe-inline` because Leptos emits its hydration
/// bootstrap, and this app its theme-init script, as inline `<script>`
/// elements. Tightening that to nonces means enabling leptos's `nonce` feature
/// and threading `use_nonce()` through the shell. It is the one meaningful
/// hardening step left in this stack, and it is written down in
/// `docs/architecture.md` rather than left as a silent gap.
const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; \
     script-src 'self' 'wasm-unsafe-eval' 'unsafe-inline'; \
     style-src 'self' 'unsafe-inline'; \
     img-src 'self' data:; \
     font-src 'self'; \
     connect-src 'self'; \
     base-uri 'none'; \
     form-action 'self'; \
     frame-ancestors 'none'; \
     object-src 'none'";

/// State shared by the Axum router.
///
/// `FromRef` lets each handler extract only the piece it needs -- `AppState`
/// for the health probe, `LeptosOptions` for the renderer -- rather than every
/// handler taking the whole thing.
#[derive(Debug, Clone, FromRef)]
pub struct ServerState {
    pub app: AppState,
    pub leptos_options: LeptosOptions,
}

/// Assemble the application router.
pub fn build(app_state: AppState, leptos_options: LeptosOptions) -> Router {
    let routes: Vec<AxumRouteListing> = generate_route_list(App);
    let is_production = app_state.config.environment.is_production();

    let state = ServerState {
        app: app_state.clone(),
        leptos_options: leptos_options.clone(),
    };

    // Runs once per request, before rendering and before any server function
    // body. This is what lets `#[server]` functions reach the database without
    // the UI crate knowing anything about how the server is wired.
    let provide_app_state = {
        let app_state = app_state.clone();
        move || provide_context(app_state.clone())
    };

    // Two years for HSTS, matching the preload-list requirement. Production
    // only: sending it over plain http://localhost would pin a developer's
    // browser to HTTPS for localhost across every project on the machine.
    let hsts = is_production
        .then(|| HeaderValue::from_static("max-age=63072000; includeSubDomains"))
        .map(|value| {
            SetResponseHeaderLayer::overriding(http::header::STRICT_TRANSPORT_SECURITY, value)
        });

    Router::new()
        // Probes are registered before the Leptos catch-all so they keep
        // answering even if the renderer is unhappy.
        .route("/health/live", get(crate::health::live))
        .route("/health/ready", get(crate::health::ready))
        // Registers both the SSR page routes and every `#[server]` endpoint,
        // each with the context provided above.
        .leptos_routes_with_context(&state, routes, provide_app_state, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        // Static assets from `site-root`, falling through to a rendered 404.
        .fallback(leptos_axum::file_and_error_handler::<ServerState, _>(shell))
        .layer(
            ServiceBuilder::new()
                // A panic in one handler becomes a 500 for that request rather
                // than a dropped connection the client reads as a network fault.
                .layer(CatchPanicLayer::new())
                // Correlates every log line for a request, and echoes the id
                // back so a user can quote it in a bug report.
                .layer(SetRequestIdLayer::new(REQUEST_ID_HEADER, MakeRequestUuid))
                .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER))
                .layer(
                    TraceLayer::new_for_http().on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(LatencyUnit::Millis),
                    ),
                )
                .layer(CompressionLayer::new().br(true).gzip(true))
                .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
                // Security headers. The previous deployment served the SPA from
                // an nginx config that set none of these.
                .layer(SetResponseHeaderLayer::overriding(
                    http::header::CONTENT_SECURITY_POLICY,
                    HeaderValue::from_static(CONTENT_SECURITY_POLICY),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    http::header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("x-frame-options"),
                    HeaderValue::from_static("DENY"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    http::header::REFERRER_POLICY,
                    HeaderValue::from_static("strict-origin-when-cross-origin"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("permissions-policy"),
                    HeaderValue::from_static("camera=(), geolocation=(), microphone=()"),
                ))
                .option_layer(hsts)
                // Innermost, for two reasons. It has to sit inside the header
                // and compression layers so a timed-out response still carries
                // the security headers; and `TimeoutLayer` needs the inner
                // response body to implement `Default` in order to synthesise
                // its 408, which the wrapped bodies from compression and the
                // body limit do not.
                .layer(TimeoutLayer::with_status_code(
                    http::StatusCode::REQUEST_TIMEOUT,
                    REQUEST_TIMEOUT,
                )),
        )
        .with_state(state)
}
