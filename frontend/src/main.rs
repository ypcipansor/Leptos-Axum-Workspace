use leptos::prelude::*;
use shared_lib::APP_NAME;

#[component]
fn App() -> impl IntoView {
    view! {
        <main>
            <h1>{APP_NAME}</h1>
            <p>"Template sistem informasi manajemen sederhana siap pakai."</p>
        </main>
    }
}

fn main() {
    let _ = view! { <App /> };
    println!("{} frontend scaffold is ready", APP_NAME);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_is_exposed_in_frontend() {
        assert_eq!(APP_NAME, "Simple Management Information System");
    }
}
