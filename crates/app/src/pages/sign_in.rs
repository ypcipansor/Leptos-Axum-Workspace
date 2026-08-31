use app_core::{PASSWORD_MAX_LEN, USERNAME_MAX_LEN};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

use crate::components::{Alert, Button, Card, TextField, ThemeToggle};
use crate::server::SignIn;

/// Sign-in.
///
/// Note what is deliberately absent: the live per-field validation the sign-up
/// page has. Telling someone their username is "too short to be valid" while
/// they are trying to sign in leaks which names exist, and the server answers
/// every failure identically for the same reason.
#[component]
pub fn SignInPage() -> impl IntoView {
    let action = ServerAction::<SignIn>::new();

    let submit_error = Memo::new(move |_| match action.value().get() {
        Some(Err(error)) => Some(error.user_message()),
        _ => None,
    });

    view! {
        <Title text="Sign in" />

        <main class="relative mx-auto flex min-h-dvh w-full max-w-md flex-col justify-center gap-6 px-4 py-12">
            <div class="absolute right-4 top-4">
                <ThemeToggle />
            </div>
            <header class="text-center">
                <h1 class="text-2xl font-semibold text-body">"Sign in"</h1>
                <p class="mt-1 text-sm text-muted">"Welcome back."</p>
            </header>

            <Card>
                <ActionForm action=action attr:class="flex flex-col gap-4">
                    <Alert message=submit_error />

                    <TextField
                        id="username"
                        name="username"
                        label="Username"
                        autocomplete="username"
                        maxlength=USERNAME_MAX_LEN
                    />

                    <TextField
                        id="password"
                        name="password"
                        label="Password"
                        input_type="password"
                        autocomplete="current-password"
                        maxlength=PASSWORD_MAX_LEN
                    />

                    <Button pending=action.pending()>"Sign in"</Button>
                </ActionForm>
            </Card>

            <p class="text-center text-sm text-muted">
                "No account yet? "
                <A href="/signup" attr:class="font-medium text-accent hover:underline">
                    "Create one"
                </A>
            </p>
        </main>
    }
}
