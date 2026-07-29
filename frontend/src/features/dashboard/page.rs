use leptos::prelude::*;

use crate::routes::AppRoute;

#[component]
pub fn DashboardPage(route: AppRoute) -> impl IntoView {
    view! {
        <p data-route=route.path()>{route.template_text()}</p>
    }
}
