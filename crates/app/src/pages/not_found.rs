use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

/// Shown for any unmatched route.
///
/// The previous hand-rolled router had no concept of an unknown route at all --
/// an unrecognised path simply rendered the dashboard shell with nothing in it.
#[component]
pub fn NotFoundPage() -> impl IntoView {
    // On the server, report it as a 404 rather than a 200 containing an
    // apology. Crawlers and uptime checks read the status, not the prose.
    #[cfg(feature = "ssr")]
    {
        if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
            response.set_status(http::StatusCode::NOT_FOUND);
        }
    }

    view! {
        <Title text="Page not found" />

        <main class="mx-auto flex min-h-dvh w-full max-w-md flex-col items-center justify-center gap-4 px-4 text-center">
            <p class="text-sm font-medium text-accent">"404"</p>
            <h1 class="text-2xl font-semibold text-body">"Page not found"</h1>
            <p class="text-sm text-muted">
                "The page you were looking for does not exist or has moved."
            </p>
            <A
                href="/"
                attr:class="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-on-accent hover:bg-accent-strong"
            >
                "Back to the dashboard"
            </A>
        </main>
    }
}
