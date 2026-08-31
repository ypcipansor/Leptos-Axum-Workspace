use leptos::prelude::*;
use leptos_meta::{HashedStylesheet, Meta, MetaTags, Title, provide_meta_context};
use leptos_router::SsrMode;
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;

use crate::components::{Alert, THEME_INIT_SCRIPT};
use crate::pages::{DashboardPage, NotFoundPage, SignInPage, SignUpPage};
use crate::server::current_user;

/// The full HTML document, rendered on the server.
///
/// This replaces the static `index.html` the previous client-only build served.
/// Because the document is produced by the same code that renders the app, the
/// two can never disagree about what scripts, styles or metadata a page needs.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />

                // Blocking and first: it decides the colour scheme before the
                // browser paints. Anything later shows a white flash.
                <script>{THEME_INIT_SCRIPT}</script>

                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
                <HashedStylesheet options id="leptos" />
                <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
                <MetaTags />
            </head>
            <body class="bg-surface-sunken text-body antialiased">
                <App />
            </body>
        </html>
    }
}

/// The application shell and route table.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title formatter=|text: String| format!("{text} · {}", app_core::APP_NAME) />
        <Meta name="description" content="A production-ready Leptos + Axum starter." />
        <Meta name="color-scheme" content="light dark" />

        // Real URL-based routing: deep links, the back button and bookmarks all
        // work. The previous implementation tracked the current screen in a
        // signal, so none of them did.
        <Router>
            <Routes fallback=NotFoundPage>
                // `Async` waits for the auth and session resources on the
                // server and renders the resolved `<Suspense>` content inline
                // in the initial HTML. The default out-of-order mode streams
                // that content in a `<template>` that only the hydration
                // runtime splices into the DOM, so with JavaScript disabled the
                // dashboard never appears -- which is what the no-JS e2e suite
                // asserts against. The server and client trees stay
                // structurally identical, so hydration does not duplicate.
                <Route path=path!("/") view=HomeRoute ssr=SsrMode::Async />
                <Route path=path!("/signin") view=SignInPage />
                <Route path=path!("/signup") view=SignUpPage />
            </Routes>
        </Router>
    }
}

/// The authenticated landing page, or a redirect to sign-in.
///
/// The check happens on the server during the initial render, so an
/// unauthenticated visitor is redirected before any dashboard markup is
/// produced -- they never receive a frame of content they are not entitled to.
#[component]
fn HomeRoute() -> impl IntoView {
    let user = Resource::new(|| (), |()| current_user());

    view! {
        <Transition fallback=PageSkeleton>
            {move || Suspend::new(async move {
                match user.await {
                    Ok(Some(profile)) => view! { <DashboardPage user=profile /> }.into_any(),
                    Ok(None) => view! { <Redirect path="/signin" /> }.into_any(),
                    Err(error) => {
                        let message = error.user_message();
                        view! {
                            <main class="mx-auto flex min-h-dvh max-w-md items-center px-4">
                                <Alert message=Signal::derive(move || Some(message.clone())) />
                            </main>
                        }
                            .into_any()
                    }
                }
            })}
        </Transition>
    }
}

/// Shown while the initial authentication check resolves.
#[component]
fn PageSkeleton() -> impl IntoView {
    view! {
        <div class="flex min-h-dvh items-center justify-center" aria-hidden="true">
            <div class="h-8 w-40 animate-pulse rounded-lg bg-surface-raised" />
        </div>
    }
}
