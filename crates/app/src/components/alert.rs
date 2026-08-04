use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Error,
    Success,
    Info,
}

impl AlertKind {
    const fn classes(self) -> &'static str {
        match self {
            Self::Error => {
                "border-danger-300 bg-danger-50 text-danger-900 \
                 dark:border-danger-800 dark:bg-danger-950 dark:text-danger-100"
            }
            Self::Success => {
                "border-success-300 bg-success-50 text-success-900 \
                 dark:border-success-800 dark:bg-success-950 dark:text-success-100"
            }
            Self::Info => {
                "border-subtle bg-surface-raised text-body \
                 dark:border-subtle dark:bg-surface-raised dark:text-body"
            }
        }
    }

    /// ARIA role. Errors get `alert`, which interrupts a screen reader
    /// immediately; the rest get `status`, which waits for a pause.
    const fn role(self) -> &'static str {
        match self {
            Self::Error => "alert",
            Self::Success | Self::Info => "status",
        }
    }

    const fn live(self) -> &'static str {
        match self {
            Self::Error => "assertive",
            Self::Success | Self::Info => "polite",
        }
    }
}

/// A message banner that assistive technology actually announces.
///
/// The previous implementation rendered error text into a plain `<div>`, so a
/// screen reader user submitting a form heard nothing at all -- the text
/// appeared visually and was silently skipped.
///
/// The wrapper is always in the DOM, even with no message. A live region has to
/// exist *before* its content changes for the change to be announced; mounting
/// the element and its text at the same moment is the single most common way to
/// get this wrong.
#[component]
pub fn Alert(
    /// The message. `None` renders an empty, still-present live region.
    #[prop(into)]
    message: Signal<Option<String>>,
    #[prop(default = AlertKind::Error)] kind: AlertKind,
) -> impl IntoView {
    view! {
        <div role=kind.role() aria-live=kind.live() aria-atomic="true">
            {move || {
                message
                    .get()
                    .map(|text| {
                        view! {
                            <p class=format!(
                                "rounded-lg border px-4 py-3 text-sm {}",
                                kind.classes(),
                            )>{text}</p>
                        }
                    })
            }}
        </div>
    }
}
