use leptos::prelude::*;

use crate::components::Spinner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
}

impl ButtonKind {
    const fn classes(self) -> &'static str {
        match self {
            Self::Primary => {
                "bg-accent text-on-accent hover:bg-accent-strong \
                 focus-visible:outline-accent"
            }
            Self::Secondary => {
                "border border-subtle bg-surface-raised text-body \
                 hover:bg-surface-sunken focus-visible:outline-accent"
            }
            Self::Danger => {
                "bg-danger-600 text-white hover:bg-danger-700 \
                 focus-visible:outline-danger-600"
            }
        }
    }
}

/// A button with a built-in pending state.
///
/// While `pending` is set the button is disabled and shows a spinner, and
/// `aria-busy` tells assistive technology the work is still running. Wiring
/// that per call site is how double-submitted forms happen.
#[component]
pub fn Button(
    #[prop(default = ButtonKind::Primary)] kind: ButtonKind,
    #[prop(into, optional)] pending: Signal<bool>,
    #[prop(into, optional)] disabled: Signal<bool>,
    #[prop(into, default = "submit".to_owned())] button_type: String,
    #[prop(into, optional)] class: String,
    children: Children,
) -> impl IntoView {
    let is_disabled = move || pending.get() || disabled.get();

    view! {
        <button
            type=button_type
            disabled=is_disabled
            aria-busy=move || if pending.get() { "true" } else { "false" }
            class=format!(
                "inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2 \
                 text-sm font-medium transition-colors focus-visible:outline-2 \
                 focus-visible:outline-offset-2 disabled:cursor-not-allowed \
                 disabled:opacity-60 {} {}",
                kind.classes(),
                class,
            )
        >
            <Show when=move || pending.get()>
                <Spinner />
            </Show>
            {children()}
        </button>
    }
}
