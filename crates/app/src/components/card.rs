use leptos::prelude::*;

/// A surface panel.
#[component]
pub fn Card(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    view! {
        <div class=format!(
            "rounded-xl border border-subtle bg-surface-raised p-6 shadow-sm {class}",
        )>{children()}</div>
    }
}
