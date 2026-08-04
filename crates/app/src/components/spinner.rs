use leptos::prelude::*;

/// A busy indicator.
///
/// `aria-hidden` because it carries no information of its own -- the state it
/// signals is already announced by the `aria-busy` on the control that owns it.
/// Without this a screen reader would read out a meaningless graphic on every
/// pending action.
#[component]
pub fn Spinner(#[prop(into, optional)] class: String) -> impl IntoView {
    view! {
        <svg
            class=format!("size-4 animate-spin {class}")
            viewBox="0 0 24 24"
            fill="none"
            aria-hidden="true"
            focusable="false"
        >
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
            <path
                class="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 0 1 8-8v4a4 4 0 0 0-4 4H4z"
            />
        </svg>
    }
}
