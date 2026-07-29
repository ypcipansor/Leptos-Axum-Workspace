use crate::routes::AppRoute;
use crate::utils::remove_local_storage_item;
use gloo_net::http::Request;
use leptos::prelude::*;
use shared_lib::SessionInfo;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn DashboardPage(
    token: ReadSignal<Option<String>>,
    username: ReadSignal<Option<String>>,
    set_token: WriteSignal<Option<String>>,
    set_username: WriteSignal<Option<String>>,
    set_route: WriteSignal<AppRoute>,
) -> impl IntoView {
    let (sessions, set_sessions) = signal(Vec::<SessionInfo>::new());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    let (success_msg, set_success_msg) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);

    let fetch_sessions = move || {
        let Some(t) = token.get() else {
            return;
        };
        set_loading.set(true);
        set_error_msg.set(None);

        spawn_local(async move {
            match Request::get("/api/sessions")
                .header("Authorization", &format!("Bearer {}", t))
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.ok() {
                        if let Ok(list) = resp.json::<Vec<SessionInfo>>().await {
                            set_sessions.set(list);
                        } else {
                            set_error_msg
                                .set(Some("Failed to parse active sessions list.".to_string()));
                        }
                    } else if resp.status() == 401 {
                        remove_local_storage_item("token");
                        remove_local_storage_item("username");
                        set_token.set(None);
                        set_username.set(None);
                        set_route.set(AppRoute::Login);
                    } else {
                        set_error_msg.set(Some("Server failed to list sessions.".to_string()));
                    }
                }
                Err(_) => {
                    set_error_msg.set(Some(
                        "Cannot connect to server to fetch sessions.".to_string(),
                    ));
                }
            }
            set_loading.set(false);
        });
    };

    Effect::new(move |_| {
        fetch_sessions();
    });

    let handle_logout = move |_| {
        remove_local_storage_item("token");
        remove_local_storage_item("username");
        set_token.set(None);
        set_username.set(None);
        set_route.set(AppRoute::Login);
    };

    let handle_revoke = move |target_token: String| {
        let Some(t) = token.get() else {
            return;
        };
        set_error_msg.set(None);
        set_success_msg.set(None);

        #[derive(serde::Serialize)]
        struct RevokePayload {
            token: String,
        }

        spawn_local(async move {
            match Request::post("/api/sessions/revoke")
                .header("Authorization", &format!("Bearer {}", t))
                .json(&RevokePayload {
                    token: target_token.clone(),
                })
                .expect("Failed to serialize revoke request")
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.ok() {
                        set_success_msg.set(Some("Session revoked successfully!".to_string()));

                        if target_token == t {
                            remove_local_storage_item("token");
                            remove_local_storage_item("username");
                            set_token.set(None);
                            set_username.set(None);
                            set_route.set(AppRoute::Login);
                        } else {
                            fetch_sessions();
                        }
                    } else {
                        set_error_msg.set(Some("Failed to revoke the session.".to_string()));
                    }
                }
                Err(_) => {
                    set_error_msg.set(Some("Failed to connect to server.".to_string()));
                }
            }
        });
    };

    view! {
        <div class="min-h-screen bg-gray-50 pb-12">
            <nav class="bg-indigo-600 shadow">
                <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="flex justify-between h-16 items-center">
                        <div class="flex-shrink-0 flex items-center">
                            <span class="text-white font-bold text-xl">"SIM Simple"</span>
                        </div>
                        <div class="flex items-center space-x-4">
                            <span id="welcome-username" class="text-white text-sm font-medium">
                                "Welcome, " {move || username.get().unwrap_or_default()}
                            </span>
                            <button
                                id="logout-btn"
                                on:click=handle_logout
                                class="bg-indigo-700 hover:bg-indigo-800 text-white px-3 py-2 rounded-md text-sm font-medium transition"
                            >
                                "Logout"
                            </button>
                        </div>
                    </div>
                </div>
            </nav>

            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 mt-8">
                <div class="bg-white p-6 rounded-lg shadow border border-gray-100">
                    <div class="flex justify-between items-center pb-4 border-b border-gray-100 mb-6">
                        <div>
                            <h1 class="text-2xl font-bold text-gray-900">"Session Management"</h1>
                            <p class="text-sm text-gray-500">"Monitor and manage active session states across devices and tabs."</p>
                        </div>
                        <button
                            id="refresh-btn"
                            disabled=loading
                            on:click=move |_| fetch_sessions()
                            class="bg-gray-100 hover:bg-gray-200 text-gray-800 px-3 py-2 rounded-md text-sm font-medium transition disabled:bg-gray-50"
                        >
                            {move || if loading.get() { "Refreshing..." } else { "Refresh" }}
                        </button>
                    </div>

                    {move || error_msg.get().map(|msg| view! {
                        <div id="dash-error-banner" class="mb-4 bg-red-50 border-l-4 border-red-400 p-4 text-sm text-red-700" role="alert">
                            {msg}
                        </div>
                    })}

                    {move || success_msg.get().map(|msg| view! {
                        <div id="dash-success-banner" class="mb-4 bg-green-50 border-l-4 border-green-400 p-4 text-sm text-green-700" role="alert">
                            {msg}
                        </div>
                    })}

                    <div class="overflow-x-auto">
                        <table class="min-w-full divide-y divide-gray-200 border border-gray-100 rounded-lg overflow-hidden">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Session Token / ID"</th>
                                    <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"IP Address"</th>
                                    <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Device / User Agent"</th>
                                    <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Login Time"</th>
                                    <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Status"</th>
                                    <th scope="col" class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                                </tr>
                            </thead>
                            <tbody class="bg-white divide-y divide-gray-100">
                                {move || {
                                    let list = sessions.get();
                                    if list.is_empty() {
                                        view! {
                                            <tr>
                                                <td colspan="6" class="px-6 py-10 text-center text-sm text-gray-500">
                                                    "No active sessions found."
                                                </td>
                                            </tr>
                                        }.into_any()
                                    } else {
                                        list.into_iter().map(|session| {
                                            let s_token = session.token.clone();
                                            let s_token_disp = if s_token.len() > 10 {
                                                format!("{}...", &s_token[..10])
                                            } else {
                                                s_token.clone()
                                            };
                                            let s_ip = session.ip_address.clone().unwrap_or_else(|| "Unknown".to_string());
                                            let s_ua = session.user_agent.clone().unwrap_or_else(|| "Unknown".to_string());
                                            let s_ua_title = s_ua.clone();
                                            let s_created_at = session.created_at.clone();
                                            let is_current = session.is_current;

                                            view! {
                                                <tr class=move || if is_current { "bg-indigo-50/20" } else { "" }>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm font-mono text-gray-900">
                                                        {s_token_disp.clone()}
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                                        {s_ip.clone()}
                                                    </td>
                                                    <td class="px-6 py-4 text-sm text-gray-500 max-w-xs truncate" title=s_ua_title.clone()>
                                                        {s_ua.clone()}
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                                        {s_created_at.clone()}
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-sm">
                                                        {if is_current {
                                                            view! { <span class="px-2.5 py-0.5 rounded-full text-xs font-semibold bg-green-100 text-green-800">"Current Session"</span> }.into_any()
                                                        } else {
                                                            view! { <span class="px-2.5 py-0.5 rounded-full text-xs font-semibold bg-gray-100 text-gray-600">"Active"</span> }.into_any()
                                                        }}
                                                    </td>
                                                    <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                                        <button
                                                            id=format!("revoke-{}", s_token)
                                                            on:click=move |_| handle_revoke(s_token.clone())
                                                            class=move || if is_current {
                                                                "text-red-600 hover:text-red-900 font-semibold transition bg-red-50 hover:bg-red-100 px-3 py-1 rounded-md"
                                                            } else {
                                                                "text-gray-600 hover:text-red-600 transition bg-gray-50 hover:bg-red-50 px-3 py-1 rounded-md"
                                                            }
                                                        >
                                                            {if is_current { "Logout This Device" } else { "Revoke" }}
                                                        </button>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view().into_any()
                                    }
                                }}
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </div>
    }
}
