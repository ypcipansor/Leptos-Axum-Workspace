use app_core::{SessionSummary, UserProfile};
use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::{Alert, AlertKind, Button, ButtonKind, Card, ThemeToggle};
use crate::server::{RevokeSession, SignOut, list_sessions};

/// The signed-in landing page: profile summary and active session management.
#[component]
pub fn DashboardPage(user: UserProfile) -> impl IntoView {
    let revoke = ServerAction::<RevokeSession>::new();
    let sign_out = ServerAction::<SignOut>::new();

    // Re-reads whenever a revoke completes, so the table reflects the change
    // without a manual refresh and without hand-written refetch plumbing.
    let sessions = Resource::new(move || revoke.version().get(), |_| list_sessions());

    let revoke_error = Memo::new(move |_| match revoke.value().get() {
        Some(Err(error)) => Some(error.user_message()),
        _ => None,
    });

    let username = user.username.to_string();

    view! {
        <Title text="Dashboard" />

        <div class="min-h-dvh bg-surface-sunken">
            <header class="border-b border-subtle bg-surface-raised">
                <div class="mx-auto flex w-full max-w-4xl items-center justify-between gap-4 px-4 py-4">
                    <div>
                        <h1 class="text-lg font-semibold text-body">"Dashboard"</h1>
                        <p class="text-sm text-muted">"Signed in as " <strong>{username}</strong></p>
                    </div>

                    <div class="flex items-center gap-2">
                        <ThemeToggle />
                        <ActionForm action=sign_out>
                            <Button kind=ButtonKind::Secondary pending=sign_out.pending()>
                                "Sign out"
                            </Button>
                        </ActionForm>
                    </div>
                </div>
            </header>

            <main class="mx-auto flex w-full max-w-4xl flex-col gap-6 px-4 py-8">
                <section aria-labelledby="sessions-heading" class="flex flex-col gap-4">
                    <div>
                        <h2 id="sessions-heading" class="text-base font-semibold text-body">
                            "Active sessions"
                        </h2>
                        <p class="text-sm text-muted">
                            "Every device currently signed in to this account. \
                             Revoking a session signs that device out immediately."
                        </p>
                    </div>

                    <Alert message=revoke_error />

                    <Transition fallback=SessionsSkeleton>
                        <ErrorBoundary fallback=|errors| {
                            let message = errors
                                .get()
                                .into_iter()
                                .next().map_or_else(|| "Could not load your sessions.".to_owned(), |(_, e)| e.to_string());
                            view! {
                                <Alert
                                    kind=AlertKind::Error
                                    message=Signal::derive(move || Some(message.clone()))
                                />
                            }
                        }>
                            {move || Suspend::new(async move {
                                sessions
                                    .await
                                    .map(|rows| {
                                        view! { <SessionTable rows=rows revoke=revoke /> }
                                    })
                            })}
                        </ErrorBoundary>
                    </Transition>
                </section>
            </main>
        </div>
    }
}

#[component]
fn SessionTable(rows: Vec<SessionSummary>, revoke: ServerAction<RevokeSession>) -> impl IntoView {
    if rows.is_empty() {
        // Unreachable in practice -- viewing this page requires a session --
        // but an empty state beats a table with no rows and no explanation.
        return view! {
            <Card>
                <p class="text-sm text-muted">"No active sessions."</p>
            </Card>
        }
        .into_any();
    }

    view! {
        // Wide tables must scroll inside their own container, never make the
        // page scroll sideways on a phone. `relative` makes this the containing
        // block for the sr-only caption/action text: `sr-only` is absolutely
        // positioned, and without a positioned ancestor it would resolve against
        // <html>, escaping the clip and still widening the page on a phone.
        <div class="relative overflow-x-auto rounded-xl border border-subtle bg-surface-raised">
            <table class="w-full min-w-[42rem] text-left text-sm">
                <caption class="sr-only">"Your active sessions"</caption>
                <thead class="border-b border-subtle text-xs uppercase tracking-wide text-muted">
                    <tr>
                        <th scope="col" class="px-4 py-3 font-medium">"Device"</th>
                        <th scope="col" class="px-4 py-3 font-medium">"IP address"</th>
                        <th scope="col" class="px-4 py-3 font-medium">"Signed in"</th>
                        <th scope="col" class="px-4 py-3 font-medium">"Last seen"</th>
                        <th scope="col" class="px-4 py-3 font-medium">
                            <span class="sr-only">"Actions"</span>
                        </th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-subtle">
                    <For each=move || rows.clone() key=|row| row.id let:row>
                        <SessionRow row=row revoke=revoke />
                    </For>
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

#[component]
fn SessionRow(row: SessionSummary, revoke: ServerAction<RevokeSession>) -> impl IntoView {
    let id = row.id;
    let is_current = row.is_current;

    let device = row
        .user_agent
        .clone()
        .unwrap_or_else(|| "Unknown device".to_owned());

    // `None` means the address could not be established from a trusted source.
    // Saying so is more useful than printing a fabricated one.
    let ip = row
        .ip_address
        .clone()
        .unwrap_or_else(|| "Not recorded".to_owned());

    view! {
        <tr class="text-body">
            <td class="px-4 py-3">
                <div class="flex items-center gap-2">
                    <span class="line-clamp-2 max-w-xs break-words">{device}</span>
                    <Show when=move || is_current>
                        <span class="rounded-full bg-success-100 px-2 py-0.5 text-xs font-medium text-success-800 dark:bg-success-900 dark:text-success-100">
                            "This device"
                        </span>
                    </Show>
                </div>
            </td>
            <td class="px-4 py-3 font-mono text-xs">{ip}</td>
            <td class="px-4 py-3">
                <Timestamp value=row.created_at />
            </td>
            <td class="px-4 py-3">
                <Timestamp value=row.last_seen_at />
            </td>
            <td class="px-4 py-3 text-right">
                <ActionForm action=revoke>
                    <input type="hidden" name="session_id" value=id.to_string() />
                    <Button
                        kind=ButtonKind::Danger
                        pending=revoke.pending()
                        class="px-3 py-1.5 text-xs"
                    >
                        {if is_current { "Sign out here" } else { "Revoke" }}
                    </Button>
                </ActionForm>
            </td>
        </tr>
    }
}

/// Renders a timestamp in a machine-readable `<time>` element.
///
/// The `datetime` attribute carries the exact RFC 3339 instant while the text
/// stays readable, which is what lets assistive technology and other tools read
/// the value unambiguously.
#[component]
fn Timestamp(value: chrono::DateTime<chrono::Utc>) -> impl IntoView {
    view! {
        <time datetime=value.to_rfc3339() class="whitespace-nowrap text-muted">
            {value.format("%Y-%m-%d %H:%M UTC").to_string()}
        </time>
    }
}

/// Placeholder shown while the session list loads.
#[component]
fn SessionsSkeleton() -> impl IntoView {
    view! {
        <div
            class="flex flex-col gap-2 rounded-xl border border-subtle bg-surface-raised p-4"
            aria-hidden="true"
        >
            {(0..3)
                .map(|_| {
                    view! { <div class="h-10 animate-pulse rounded-lg bg-surface-sunken" /> }
                })
                .collect_view()}
        </div>
    }
}
