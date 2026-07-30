use crate::features::auth::{LoginPage, RegisterPage};
use crate::features::dashboard::DashboardPage;
use crate::routes::AppRoute;
use crate::utils::get_local_storage_item;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn App() -> impl IntoView {
    // Initial State from localStorage
    let initial_token = get_local_storage_item("token");
    let initial_username = get_local_storage_item("username");

    let (token, set_token) = signal(initial_token.clone());
    let (username, set_username) = signal(initial_username);

    // Initial Route determination
    let initial_route = if initial_token.is_some() {
        AppRoute::Dashboard
    } else {
        AppRoute::Login
    };
    let (route, set_route) = signal(initial_route);

    // Cross-tab synchronization via localStorage events
    Effect::new(move |_| {
        if let Some(win) = web_sys::window() {
            let token_cb = wasm_bindgen::prelude::Closure::<dyn FnMut(web_sys::StorageEvent)>::new(
                move |ev: web_sys::StorageEvent| {
                    if let Some(key) = ev.key() {
                        if key == "token" {
                            let new_val = ev.new_value();
                            set_token.set(new_val.clone());
                            if new_val.is_none() {
                                set_route.set(AppRoute::Login);
                                set_username.set(None);
                            } else {
                                let current_uname = get_local_storage_item("username");
                                set_username.set(current_uname);
                                set_route.set(AppRoute::Dashboard);
                            }
                        } else if key == "username" {
                            set_username.set(ev.new_value());
                        }
                    }
                },
            );

            win.add_event_listener_with_callback("storage", token_cb.as_ref().unchecked_ref())
                .expect("Failed to add storage listener");

            token_cb.forget();
        }
    });

    view! {
        <main class="min-h-screen bg-gray-50 text-gray-900 font-sans antialiased">
            {move || {
                let current_route = route.get();
                let has_token = token.get().is_some();

                if !has_token {
                    // Not logged in: can only access Register or Login
                    match current_route {
                        AppRoute::Register => {
                            view! {
                                <RegisterPage
                                    set_route=set_route
                                />
                            }.into_any()
                        }
                        _ => {
                            view! {
                                <LoginPage
                                    set_route=set_route
                                    set_token=set_token
                                    set_username=set_username
                                />
                            }.into_any()
                        }
                    }
                } else {
                    // Logged in: render DashboardPage
                    view! {
                        <DashboardPage
                            token=token
                            username=username
                            set_token=set_token
                            set_username=set_username
                            set_route=set_route
                        />
                    }.into_any()
                }
            }}
        </main>
    }
}
