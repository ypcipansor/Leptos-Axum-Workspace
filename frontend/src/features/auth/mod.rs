use crate::routes::AppRoute;
use crate::utils::set_local_storage_item;
use gloo_net::http::Request;
use leptos::prelude::*;
use shared_lib::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse};
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn LoginPage(
    set_route: WriteSignal<AppRoute>,
    set_token: WriteSignal<Option<String>>,
    set_username: WriteSignal<Option<String>>,
) -> impl IntoView {
    let (username_input, set_username_input) = signal(String::new());
    let (password_input, set_password_input) = signal(String::new());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    let (success_msg, set_success_msg) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);

    let handle_submit = move || {
        let u = username_input.get();
        let p = password_input.get();

        println!("LOGIN ACTION triggered: username='{}', password='{}'", u, p);

        if u.is_empty() || p.is_empty() {
            set_error_msg.set(Some("Username and password cannot be empty.".to_string()));
            return;
        }

        set_loading.set(true);
        set_error_msg.set(None);
        set_success_msg.set(None);

        spawn_local(async move {
            let req = LoginRequest {
                username: u,
                password: p,
            };

            match Request::post("/api/auth/login")
                .json(&req)
                .expect("Failed to serialize login request")
                .send()
                .await
            {
                Ok(resp) => {
                    println!("LOGIN ACTION response status={}", resp.status());
                    if resp.ok() {
                        if let Ok(body) = resp.json::<LoginResponse>().await {
                            if body.success {
                                let token = body.token.expect("missing token in response");
                                let uname = body.username.expect("missing username in response");

                                set_local_storage_item("token", &token);
                                set_local_storage_item("username", &uname);

                                set_token.set(Some(token));
                                set_username.set(Some(uname));
                                set_success_msg.set(Some("Logged in successfully!".to_string()));
                                set_route.set(AppRoute::Dashboard);
                            } else {
                                set_error_msg.set(Some(body.message));
                            }
                        } else {
                            set_error_msg
                                .set(Some("Failed to parse response from server.".to_string()));
                        }
                    } else if resp.status() == 401 {
                        set_error_msg.set(Some("Invalid username or password.".to_string()));
                    } else {
                        set_error_msg.set(Some("An error occurred on the server.".to_string()));
                    }
                }
                Err(_) => {
                    set_error_msg.set(Some(
                        "Cannot connect to server. Is the backend running?".to_string(),
                    ));
                }
            }
            set_loading.set(false);
        });
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            handle_submit();
        }
    };

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8 bg-white p-8 rounded-xl shadow-lg border border-gray-100">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
                        "Sign In to SIM"
                    </h2>
                    <p class="mt-2 text-center text-sm text-gray-600">
                        "Simple Management Information System"
                    </p>
                </div>

                {move || error_msg.get().map(|msg| view! {
                    <div id="error-banner" class="bg-red-50 border-l-4 border-red-400 p-4 text-sm text-red-700" role="alert">
                        {msg}
                    </div>
                })}

                {move || success_msg.get().map(|msg| view! {
                    <div id="success-banner" class="bg-green-50 border-l-4 border-green-400 p-4 text-sm text-green-700" role="alert">
                        {msg}
                    </div>
                })}

                <div class="mt-8 space-y-6">
                    <div class="rounded-md shadow-sm space-y-4">
                        <div>
                            <label for="username" class="block text-sm font-medium text-gray-700">"Username"</label>
                            <input
                                id="username"
                                type="text"
                                required
                                class="mt-1 appearance-none rounded-md relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 focus:z-10 sm:text-sm"
                                placeholder="Enter your username"
                                prop:value=username_input
                                on:input=move |ev| set_username_input.set(event_target_value(&ev))
                                on:keydown=on_keydown.clone()
                            />
                        </div>
                        <div>
                            <label for="password" class="block text-sm font-medium text-gray-700">"Password"</label>
                            <input
                                id="password"
                                type="password"
                                required
                                class="mt-1 appearance-none rounded-md relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 focus:z-10 sm:text-sm"
                                placeholder="Enter your password"
                                prop:value=password_input
                                on:input=move |ev| set_password_input.set(event_target_value(&ev))
                                on:keydown=on_keydown
                            />
                        </div>
                    </div>

                    <div>
                        <button
                            id="login-btn"
                            type="button"
                            disabled=loading
                            on:click=move |_| handle_submit()
                            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:bg-indigo-400 transition"
                        >
                            {move || if loading.get() { "Signing in..." } else { "Sign In" }}
                        </button>
                    </div>
                </div>

                <div class="text-center mt-4">
                    <p class="text-sm text-gray-600">
                        "Don't have an account? "
                        <button
                            id="go-to-register"
                            class="font-medium text-indigo-600 hover:text-indigo-500 outline-none"
                            on:click=move |_| set_route.set(AppRoute::Register)
                        >
                            "Register here"
                        </button>
                    </p>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn RegisterPage(set_route: WriteSignal<AppRoute>) -> impl IntoView {
    let (username_input, set_username_input) = signal(String::new());
    let (password_input, set_password_input) = signal(String::new());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    let (success_msg, set_success_msg) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);

    let handle_submit = move || {
        let u = username_input.get();
        let p = password_input.get();

        println!(
            "REGISTER ACTION triggered: username='{}', password='{}'",
            u, p
        );

        if u.is_empty() || p.len() < 4 {
            set_error_msg.set(Some(
                "Username cannot be empty and password must be at least 4 characters.".to_string(),
            ));
            return;
        }

        set_loading.set(true);
        set_error_msg.set(None);
        set_success_msg.set(None);

        spawn_local(async move {
            let req = RegisterRequest {
                username: u,
                password: p,
            };

            match Request::post("/api/auth/register")
                .json(&req)
                .expect("Failed to serialize register request")
                .send()
                .await
            {
                Ok(resp) => {
                    println!("REGISTER ACTION response status={}", resp.status());
                    if resp.ok() {
                        set_success_msg.set(Some(
                            "Registration successful! You can now log in.".to_string(),
                        ));
                        set_username_input.set(String::new());
                        set_password_input.set(String::new());
                    } else {
                        if let Ok(body) = resp.json::<RegisterResponse>().await {
                            set_error_msg.set(Some(body.message));
                        } else {
                            set_error_msg.set(Some("Failed to parse server response.".to_string()));
                        }
                    }
                }
                Err(_) => {
                    set_error_msg.set(Some(
                        "Cannot connect to server. Is backend running?".to_string(),
                    ));
                }
            }
            set_loading.set(false);
        });
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            handle_submit();
        }
    };

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8 bg-white p-8 rounded-xl shadow-lg border border-gray-100">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
                        "Create Account"
                    </h2>
                    <p class="mt-2 text-center text-sm text-gray-600">
                        "Register for the Simple Management Information System"
                    </p>
                </div>

                {move || error_msg.get().map(|msg| view! {
                    <div id="error-banner" class="bg-red-50 border-l-4 border-red-400 p-4 text-sm text-red-700" role="alert">
                        {msg}
                    </div>
                })}

                {move || success_msg.get().map(|msg| view! {
                    <div id="success-banner" class="bg-green-50 border-l-4 border-green-400 p-4 text-sm text-green-700" role="alert">
                        {msg}
                    </div>
                })}

                <div class="mt-8 space-y-6">
                    <div class="rounded-md shadow-sm space-y-4">
                        <div>
                            <label for="reg-username" class="block text-sm font-medium text-gray-700">"Username"</label>
                            <input
                                id="reg-username"
                                type="text"
                                required
                                class="mt-1 appearance-none rounded-md relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 focus:z-10 sm:text-sm"
                                placeholder="Choose a username"
                                prop:value=username_input
                                on:input=move |ev| set_username_input.set(event_target_value(&ev))
                                on:keydown=on_keydown.clone()
                            />
                        </div>
                        <div>
                            <label for="reg-password" class="block text-sm font-medium text-gray-700">"Password"</label>
                            <input
                                id="reg-password"
                                type="password"
                                required
                                class="mt-1 appearance-none rounded-md relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 focus:z-10 sm:text-sm"
                                placeholder="Choose a password (min 4 chars)"
                                prop:value=password_input
                                on:input=move |ev| set_password_input.set(event_target_value(&ev))
                                on:keydown=on_keydown
                            />
                        </div>
                    </div>

                    <div>
                        <button
                            id="register-btn"
                            type="button"
                            disabled=loading
                            on:click=move |_| handle_submit()
                            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:bg-indigo-400 transition"
                        >
                            {move || if loading.get() { "Registering..." } else { "Register" }}
                        </button>
                    </div>
                </div>

                <div class="text-center mt-4">
                    <p class="text-sm text-gray-600">
                        "Already have an account? "
                        <button
                            id="go-to-login"
                            class="font-medium text-indigo-600 hover:text-indigo-500 outline-none"
                            on:click=move |_| set_route.set(AppRoute::Login)
                        >
                            "Sign In here"
                        </button>
                    </p>
                </div>
            </div>
        </div>
    }
}
