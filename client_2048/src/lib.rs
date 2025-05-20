use crate::agent::{Postcard, StorageTask};
use crate::at_repo_sync::AtRepoSync;
use crate::components::theme_picker::ThemePicker;
use crate::idb::{DB_NAME, SESSIONS_STORE, object_delete};
use crate::oauth_client::oauth_client;
use crate::pages::callback::CallbackPage;
use crate::pages::game::GamePage;
use crate::pages::history::HistoryPage;
use crate::pages::login::LoginPage;
use crate::pages::seed::SeedPage;
use crate::pages::stats::StatsPage;
use crate::store::UserStore;
use atrium_api::agent::Agent;
use gloo_utils::document;
use indexed_db_futures::database::Database;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlInputElement};
use yew::platform::spawn_local;
use yew::prelude::*;
use yew_agent::oneshot::OneshotProvider;
use yew_hooks::use_effect_once;
use yew_router::prelude::*;
use yewdux::use_store;

pub mod agent;
pub mod at_repo_sync;
mod atrium_stores;
mod components;
pub mod idb;
pub mod oauth_client;
mod pages;
mod resolver;
pub mod store;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    GamePage,
    #[at("/login")]
    LoginPage,
    #[at("/login/:did")]
    LoginPageWithDid { did: String },
    #[at("/oauth/callback")]
    CallbackPage,
    #[at("/stats")]
    StatsPage,
    #[at("/seed/:seed")]
    SeedPage { seed: u32 },
    #[at("/seed")]
    SeedPageNoSeed,
    #[at("/history")]
    HistoryPage,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::GamePage => html! { <GamePage /> },
        Route::LoginPage => html! { <LoginPage did={None::<String>} /> },
        Route::LoginPageWithDid { did } => html! { <LoginPage did={Some(did)} /> },
        Route::CallbackPage => html! { <CallbackPage /> },
        Route::StatsPage => html! { <StatsPage /> },
        Route::SeedPage { seed } => html! { <SeedPage starting_seed={seed} /> },
        Route::SeedPageNoSeed => html! { <SeedPage starting_seed={None} /> },
        Route::HistoryPage => {
            html! { <HistoryPage /> }
        }
        Route::NotFound => html! { <h1>{ "404" }</h1> },
    }
}

fn check_drawer_open() {
    match document().get_element_by_id("my-drawer-3") {
        None => {}
        Some(element) => {
            let checkbox: HtmlInputElement = element.unchecked_into();
            checkbox.set_checked(false);
        }
    }
}

#[function_component]
fn Main() -> Html {
    let (user_store, dispatch) = use_store::<UserStore>();

    let menu_entry_onclick = Callback::from(move |_: MouseEvent| {
        check_drawer_open();
    });

    let _submenu_entry_onclick = Callback::from(move |_: MouseEvent| {
        let collection = document().get_elements_by_class_name("lg:dropdown");
        for i in 0..collection.length() {
            if let Some(element) = collection.item(i) {
                let element: HtmlElement = element.unchecked_into();
                let _ = element.remove_attribute("open");
            }
        }
        check_drawer_open();
    });

    let user_store_clone = user_store.clone();
    let dispatch_clone = dispatch.clone();
    //logout callback
    let onclick = Callback::from(move |_: MouseEvent| {
        check_drawer_open();

        let user_store = user_store_clone.clone();
        let dispatch = dispatch_clone.clone();
        spawn_local(async move {
            let db = match Database::open(DB_NAME).await {
                Ok(db) => db,
                Err(err) => {
                    log::error!("{:?}", err);
                    return;
                }
            };

            if let Some(did) = user_store.did.clone() {
                dispatch.set(UserStore::default());

                object_delete(db, SESSIONS_STORE, &did)
                    .await
                    .unwrap_or_else(|err| {
                        log::error!("{:?}", err);
                    })
            }
        });
    });

    let user_store_clone = user_store.clone();
    use_effect_once(move || {
        if user_store_clone.did.is_some() {
            // Effect logic here
            spawn_local(async move {
                match user_store_clone.did.clone() {
                    None => {}
                    Some(did) => {
                        let oauth_client = oauth_client();
                        let session = match oauth_client.restore(&did).await {
                            Ok(session) => session,
                            Err(err) => {
                                log::error!("{:?}", err);
                                return;
                            }
                        };

                        let agent = Agent::new(session);
                        let at_repo_sync = AtRepoSync::new_logged_in_repo(agent, did);
                        match at_repo_sync.sync_profiles().await {
                            Ok(_) => {}
                            Err(err) => {
                                log::error!("Error syncing your profile: {:?}", err.to_string());
                            }
                        }
                        match at_repo_sync.sync_stats().await {
                            Ok(_) => {}
                            Err(err) => {
                                log::error!("Error syncing stats: {:?}", err.to_string());
                            }
                        }
                    }
                }
            });
        }

        || ()
    });

    let mut links: Vec<Html> = vec![
        html! {<li key=1 onclick={menu_entry_onclick.clone()}><Link<Route> to={Route::GamePage}>{ "Play" }</Link<Route>></li>},
        html! {<li key=2 onclick={menu_entry_onclick.clone()}><Link<Route> to={Route::StatsPage}>{ "Stats" }</Link<Route>></li>},
        html! {<li key=3 onclick={menu_entry_onclick.clone()}><Link<Route> to={Route::HistoryPage}>{ "History" }</Link<Route>></li>},
        //WIll come back to this, just buggy and clicks wrong entry sometimes
        // html! {
        // <li key={2}>
        //     <details class="lg:dropdown invisible md:visible">
        //      <summary>{
        //         match &user_store.handle {
        //             Some(handle) => handle.to_string(),
        //             None => "Profile".to_string()
        //         }
        //     }</summary>
        //      <ul class="lg:menu lg:dropdown-content lg:bg-base-100 lg:rounded-box z-1 lg:w-52 p-2 lg:shadow-sm">
        //        <li onclick={submenu_entry_onclick.clone()}><Link<Route> to={Route::StatsPage}>{ "Stats" }</Link<Route>></li>
        //        <li onclick={submenu_entry_onclick.clone()}><Link<Route> to={Route::HistoryPage}>{ "History" }</Link<Route>></li>
        //      </ul>
        //     </details>
        //    </li>
        //    },
    ];

    links.push(html! {
        <li key=4>
            <a href="https://github.com/fatfingers23/at_2048">{ "GitHub" }</a>
        </li>
    });

    if user_store.did.is_some() {
        links.push(html! {
            <li key=5>
                <a class="cursor-pointer" {onclick}>{ "Logout" }</a>
            </li>
        });
    } else {
        links.push(html! {
            <li key=5 {onclick}>
                <Link<Route> to={Route::LoginPage}>{ "Login" }</Link<Route>>
            </li>
        });
    }

    html! {
        <div class="drawer">
            <input id="my-drawer-3" type="checkbox" class="drawer-toggle" />
            <div class="drawer-content flex flex-col">
                // <!-- Navbar -->
                <div class="navbar bg-base-300 w-full">
                    <div class="flex-none lg:hidden">
                        <label
                            for="my-drawer-3"
                            aria-label="open sidebar"
                            class="btn btn-square btn-ghost"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                fill="none"
                                viewBox="0 0 24 24"
                                class="inline-block h-6 w-6 stroke-current"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M4 6h16M4 12h16M4 18h16"
                                />
                            </svg>
                        </label>
                    </div>
                    <div class="text-xl mx-2 flex-1 px-2">{ "at://2048 (alpha)" }</div>
                    <div class="hidden flex-none lg:block">
                        <ul class="menu menu-horizontal">
                            // <!-- Navbar menu content here -->
                            { links.iter().cloned().collect::<Html>() }
                        </ul>
                    </div>
                    <div class="md:block hidden">
                        <ThemePicker />
                    </div>
                </div>
                <main>
                    <Switch<Route> render={switch} />
                </main>
            </div>
            <div class="drawer-side">
                <label for="my-drawer-3" aria-label="close sidebar" class="drawer-overlay" />
                <ul class="menu bg-base-200 min-h-full w-80 p-4">
                    // <!-- Sidebar content here -->
                    { links.iter().cloned().collect::<Html>() }
                    // { for links.clone().into_iter().enumerate().map(|(i, link)| html! { <li key={i} onclick={menu_entry_onclick.clone()}>{ link }</li> }) }
                    <div class="p-4">
                        <ThemePicker />
                    </div>
                </ul>
            </div>
        </div>
    }
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <OneshotProvider<StorageTask, Postcard> path="/worker.js">
            <BrowserRouter>
                <Main />
            </BrowserRouter>
        </OneshotProvider<StorageTask, Postcard>>
    }
}
