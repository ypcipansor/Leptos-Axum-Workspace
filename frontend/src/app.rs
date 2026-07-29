use leptos::prelude::*;
use shared_lib::APP_NAME;

use crate::features::dashboard::DashboardPage;
use crate::routes::AppRoute;

#[component]
pub fn App() -> impl IntoView {
    let route = AppRoute::Dashboard;

    view! {
        <main>
            <h1>{APP_NAME}</h1>
            <DashboardPage route=route />
        </main>
    }
}
