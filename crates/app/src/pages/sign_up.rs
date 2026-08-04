use app_core::{PASSWORD_MAX_LEN, PASSWORD_MIN_LEN, Password, USERNAME_MAX_LEN, Username};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

use crate::components::{Alert, Button, Card, TextField};
use crate::server::SignUp;

/// Account creation.
///
/// The live field validation here runs [`Username::parse`] and
/// [`Password::parse`] -- the very same functions the server calls before
/// touching the database. There is no second copy of the rules to drift out of
/// sync, which is what made the previous implementation's two independent
/// validators inevitable to get wrong.
#[component]
pub fn SignUpPage() -> impl IntoView {
    let action = ServerAction::<SignUp>::new();

    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());

    // Errors appear only once a field has been touched, so the form does not
    // greet a new visitor with a wall of complaints about empty inputs.
    let username_error = Memo::new(move |_| {
        let value = username.get();
        (!value.is_empty())
            .then(|| Username::parse(&value).err())
            .flatten()
            .map(|e| e.to_string())
    });

    let password_error = Memo::new(move |_| {
        let value = password.get();
        (!value.is_empty())
            .then(|| Password::parse(&value).err())
            .flatten()
            .map(|e| e.to_string())
    });

    let submit_error = Memo::new(move |_| match action.value().get() {
        Some(Err(error)) => Some(error.user_message()),
        _ => None,
    });

    let can_submit = Memo::new(move |_| {
        Username::parse(&username.get()).is_ok() && Password::parse(&password.get()).is_ok()
    });

    view! {
        <Title text="Create an account" />

        <main class="mx-auto flex min-h-dvh w-full max-w-md flex-col justify-center gap-6 px-4 py-12">
            <header class="text-center">
                <h1 class="text-2xl font-semibold text-body">"Create an account"</h1>
                <p class="mt-1 text-sm text-muted">"It only takes a moment."</p>
            </header>

            <Card>
                // ActionForm posts to the server function as a real HTML form
                // when JavaScript is unavailable, and intercepts it for a
                // client-side round trip when it is. Same code path either way.
                <ActionForm action=action attr:class="flex flex-col gap-4" attr:novalidate="true">
                    <Alert message=submit_error />

                    <TextField
                        id="username"
                        name="username"
                        label="Username"
                        autocomplete="username"
                        hint=format!("3 to {USERNAME_MAX_LEN} characters: letters, digits, '.', '-' and '_'.")
                        maxlength=USERNAME_MAX_LEN
                        error=username_error
                        value=username
                        on_input=Callback::new(move |value| set_username.set(value))
                    />

                    <TextField
                        id="password"
                        name="password"
                        label="Password"
                        input_type="password"
                        autocomplete="new-password"
                        hint=format!("At least {PASSWORD_MIN_LEN} characters.")
                        maxlength=PASSWORD_MAX_LEN
                        minlength=PASSWORD_MIN_LEN
                        error=password_error
                        value=password
                        on_input=Callback::new(move |value| set_password.set(value))
                    />

                    <Button pending=action.pending() disabled=Signal::derive(move || !can_submit.get())>
                        "Create account"
                    </Button>
                </ActionForm>
            </Card>

            <p class="text-center text-sm text-muted">
                "Already have an account? "
                <A href="/signin" attr:class="font-medium text-accent hover:underline">
                    "Sign in"
                </A>
            </p>
        </main>
    }
}
