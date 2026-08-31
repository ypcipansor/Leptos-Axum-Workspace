use leptos::prelude::*;

/// A labelled text input wired for assistive technology.
///
/// The label is a real `<label for=...>`, the hint and error are linked through
/// `aria-describedby`, and an invalid field carries `aria-invalid`. Getting
/// this right once here is why no page has to remember it.
///
/// The input also carries a real `name`, because the forms in this app post to
/// server functions as ordinary HTML forms when JavaScript is unavailable.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn TextField(
    #[prop(into)] id: String,
    #[prop(into)] name: String,
    #[prop(into)] label: String,
    #[prop(into, default = "text".to_owned())] input_type: String,
    #[prop(into, optional)] autocomplete: Option<String>,
    #[prop(into, optional)] hint: Option<String>,
    #[prop(into, optional)] error: Signal<Option<String>>,
    #[prop(optional)] maxlength: Option<usize>,
    #[prop(optional)] minlength: Option<usize>,
    #[prop(default = true)] required: bool,
    #[prop(optional)] on_input: Option<Callback<String>>,
) -> impl IntoView {
    let hint_id = format!("{id}-hint");
    let error_id = format!("{id}-error");

    let described_by = {
        let hint_id = hint_id.clone();
        let error_id = error_id.clone();
        let has_hint = hint.is_some();
        move || {
            let mut ids = Vec::new();
            if has_hint {
                ids.push(hint_id.clone());
            }
            if error.get().is_some() {
                ids.push(error_id.clone());
            }
            // An empty aria-describedby is invalid, so omit the attribute
            // entirely rather than emitting "".
            (!ids.is_empty()).then(|| ids.join(" "))
        }
    };

    view! {
        <div class="flex flex-col gap-1.5">
            <label for=id.clone() class="text-sm font-medium text-body">
                {label}
            </label>

            <input
                id=id.clone()
                name=name
                type=input_type
                autocomplete=autocomplete
                required=required
                maxlength=maxlength.map(|v| v.to_string())
                minlength=minlength.map(|v| v.to_string())
                aria-invalid=move || error.get().map(|_| "true")
                aria-describedby=described_by
                on:input=move |ev| {
                    if let Some(handler) = on_input {
                        handler.run(event_target_value(&ev));
                    }
                }
                class="rounded-lg border border-subtle bg-surface px-3 py-2 text-sm \
                       text-body placeholder:text-muted focus-visible:outline-2 \
                       focus-visible:outline-offset-1 focus-visible:outline-accent \
                       aria-[invalid=true]:border-danger-500"
            />

            {hint
                .map(|text| {
                    view! {
                        <p id=hint_id class="text-xs text-muted">
                            {text}
                        </p>
                    }
                })}

            <p id=error_id class="text-xs text-danger-600 dark:text-danger-400 empty:hidden">
                {move || error.get()}
            </p>
        </div>
    }
}
