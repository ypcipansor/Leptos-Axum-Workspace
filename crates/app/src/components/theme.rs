use leptos::prelude::*;

/// Applies the stored colour scheme before the first paint.
///
/// This has to be a blocking inline script in `<head>`. Any later -- a Leptos
/// effect, a hydration callback -- and the browser has already painted the
/// light theme, producing a white flash on every navigation for dark-theme
/// users.
///
/// It is also why the choice lives in `localStorage` rather than in a signal:
/// the value must be readable before any framework code exists.
pub const THEME_INIT_SCRIPT: &str = r"
(function () {
  try {
    var stored = localStorage.getItem('theme');
    var dark = stored
      ? stored === 'dark'
      : window.matchMedia('(prefers-color-scheme: dark)').matches;
    document.documentElement.classList.toggle('dark', dark);
  } catch (_) {
    /* Private browsing can make localStorage throw. Fall back to light. */
  }
})();
";

/// Switches between light and dark, remembering the choice.
#[component]
pub fn ThemeToggle() -> impl IntoView {
    let toggle = move |_| toggle_theme();

    view! {
        <button
            type="button"
            on:click=toggle
            class="rounded-lg border border-subtle bg-surface-raised p-2 text-body \
                   transition-colors hover:bg-surface-sunken focus-visible:outline-2 \
                   focus-visible:outline-offset-2 focus-visible:outline-accent"
            aria-label="Toggle dark mode"
            title="Toggle dark mode"
        >
            // Exactly one of these is visible at a time, chosen by the `dark`
            // class on <html>, so the icon is correct on first paint without
            // any client state having loaded yet.
            <svg
                class="size-5 dark:hidden"
                viewBox="0 0 24 24"
                fill="currentColor"
                aria-hidden="true"
                focusable="false"
            >
                <path d="M12 3a9 9 0 1 0 9 9c0-.46-.04-.92-.1-1.36A5.39 5.39 0 0 1 12 3z" />
            </svg>
            <svg
                class="hidden size-5 dark:block"
                viewBox="0 0 24 24"
                fill="currentColor"
                aria-hidden="true"
                focusable="false"
            >
                <path d="M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10zm0-5v3m0 14v3m10-10h-3M5 12H2m15.07-7.07-2.12 2.12M9.05 14.95l-2.12 2.12m10.14 0-2.12-2.12M9.05 9.05 6.93 6.93" />
            </svg>
        </button>
    }
}

/// Flip the theme and persist it.
///
/// Only ever reached from a click handler, which cannot fire during
/// server rendering -- but the function still has to *compile* for the server
/// target, hence the `cfg`.
fn toggle_theme() {
    #[cfg(feature = "hydrate")]
    {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(root) = window.document().and_then(|d| d.document_element()) else {
            return;
        };

        let now_dark = !root.class_list().contains("dark");
        let _ = root.class_list().toggle_with_force("dark", now_dark);

        // Storage access throws in some privacy modes; a failure here should
        // change the theme for this page rather than break the button.
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("theme", if now_dark { "dark" } else { "light" });
        }
    }
}
