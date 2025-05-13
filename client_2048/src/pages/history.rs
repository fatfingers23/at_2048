use crate::Route;
use crate::agent::{StorageRequest, StorageResponse, StorageTask};
use crate::at_repo_sync::AtRepoSyncError;
use crate::idb::{DB_NAME, GAME_STORE, RecordStorageWrapper, paginated_cursor};
use crate::pages::game::TileProps;
use crate::store::UserStore;
use StorageResponse::RepoError;
use atrium_api::types::string::Did;
use gloo::dialogs::{alert, confirm};
use indexed_db_futures::database::Database;
use std::rc::Rc;
use twothousand_forty_eight::unified::game::GameState;
use twothousand_forty_eight::unified::validation::{Validatable, ValidationResult};
use twothousand_forty_eight::v2::recording::SeededRecording;
use twothousand_forty_eight::wasm::new_game;
use types_2048::blue::_2048::game;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;
use yew::platform::spawn_local;
use yew::prelude::*;
use yew_agent::oneshot::use_oneshot_runner;
use yew_hooks::use_effect_once;
use yew_router::hooks::use_navigator;
use yew_router::prelude::Link;
use yewdux::use_store;

#[derive(Clone, PartialEq, Default)]
pub enum TabState {
    #[default]
    Local,
    Remote,
    Both,
}

impl From<TabState> for String {
    fn from(tab: TabState) -> Self {
        match tab {
            TabState::Local => "Local".to_string(),
            TabState::Remote => "Remote".to_string(),
            TabState::Both => "Both".to_string(),
        }
    }
}

impl From<TabState> for &'static str {
    fn from(tab: TabState) -> Self {
        match tab {
            TabState::Local => "Local",
            TabState::Remote => "Remote",
            TabState::Both => "Both",
        }
    }
}

#[derive(Properties, Clone, PartialEq)]
struct HistoryTabProps {
    action: Callback<TabState>,
}
#[function_component(HistoryTab)]
fn tab_component(props: &HistoryTabProps) -> Html {
    let tab_state = use_state(|| TabState::default());

    let tab_event_clone = tab_state.clone();
    let action = props.action.clone();
    let onclick = Callback::from(move |event: MouseEvent| {
        let element = event.target().unwrap().dyn_into::<HtmlElement>().unwrap();
        let tab_name = element.text_content().unwrap();
        let local_tab_state = match tab_name.as_str() {
            "Local" => TabState::Local,
            "Remote" => TabState::Remote,
            "Both" => TabState::Both,
            _ => TabState::Local,
        };
        action.emit(local_tab_state.clone());
        tab_event_clone.set(local_tab_state)
    });

    html! {
        <div role="tablist" class="tabs tabs-lift tabs-lg">
            <a
                onclick={onclick.clone()}
                role="tab"
                class={classes!("tab", (*tab_state == TabState::Local).then(|| Some("tab-active")))}
            >
                { "Local" }
            </a>
            <a
                onclick={onclick}
                role="tab"
                class={classes!("tab", (*tab_state == TabState::Remote).then(|| Some("tab-active")))}
            >
                { "Remote" }
            </a>
            // <a
            //     {onclick}
            //     role="tab"
            //     class={classes!("tab", (*tab_state == TabState::Both).then(|| Some("tab-active")))}
            // >
            //     { "Both" }
            // </a>
        </div>
    }
}

#[function_component(MiniTile)]
fn mini_tile(props: &TileProps) -> Html {
    let TileProps {
        tile_value: tile_value_ref,
        ..
    } = props;

    let text = if *tile_value_ref == 0 {
        String::new()
    } else {
        tile_value_ref.to_string()
    };

    //TODO fix font size for big numbers
    let tile_class = crate::pages::game::get_bg_color_and_text_color(*tile_value_ref);
    html! {
        <div class="  p-1 flex items-center justify-center">
            <div
                class={format!(
                        "flex items-center justify-center w-8 h-full {} font-bold text rounded-md",
                        tile_class
                    )}
            >
                { text }
            </div>
        </div>
    }
}

#[derive(Properties, Clone, PartialEq)]
struct MiniGameboardProps {
    recording: SeededRecording,
}

#[function_component(MiniGameboard)]
fn mini_gameboard(props: &MiniGameboardProps) -> Html {
    let gamestate = GameState::from_reconstructable_ruleset(&props.recording).unwrap();
    let flatten_tiles = gamestate
        .board
        .tiles
        .iter()
        .flatten()
        .filter_map(|tile| *tile)
        .collect::<Vec<_>>();
    html! {
        <div
            class="w-1/4 flex-1 mx-auto w-full bg-light-board-background shadow-2xl rounded-md p-1"
        >
            <div class={classes!(String::from("grid grid-cols-4 p-1 md:p-2 w-full h-full"))}>
                { flatten_tiles.into_iter().map(|tile| {
                     html! { <MiniTile key={tile.id} tile_value={tile.value} new_tile={tile.new} x={tile.x} y={tile.y} size={4} /> }
                }).collect::<Html>() }
            </div>
        </div>
    }
}

#[derive(Properties, Clone, PartialEq)]
struct GameTileProps {
    game: Rc<RecordStorageWrapper<game::RecordData>>,
    did: Option<Did>,
    reload_action: Callback<()>,
}

#[function_component(GameTile)]
fn game_tile(props: &GameTileProps) -> Html {
    let record_key = props.game.rkey.clone();
    let game = props.game.record.clone();
    let seeded_recording = use_state(|| None);
    let validation_result: UseStateHandle<Option<ValidationResult>> = use_state(|| None);
    let resync_loading = use_state(|| false);
    let sync_error = use_state(|| None);
    let navigator = use_navigator().unwrap();

    let storage_task = use_oneshot_runner::<StorageTask>();
    let storage_agent = storage_task.clone();

    use_effect_with(seeded_recording.clone(), move |seeded_recording| match game
        .seeded_recording
        .parse::<SeededRecording>(
    ) {
        Ok(results) => seeded_recording.set(Some(results)),
        Err(err) => {
            log::error!("{:?}", err);
        }
    });

    let validation_clone = validation_result.clone();
    use_effect_with(
        seeded_recording.clone(),
        move |seeded_recording| match seeded_recording.as_ref() {
            Some(seeded_recording) => match seeded_recording.validate() {
                Ok(result) => validation_clone.set(Some(result)),
                Err(_) => {}
            },
            None => {}
        },
    );

    let storage_agent_for_click = storage_agent.clone(); // Clone it before use
    let did = props.did.clone();
    let resync_loading_clone = resync_loading.clone();
    let sync_error_clone = sync_error.clone();
    let cloned_reload_action = props.reload_action.clone();
    let sync_onclick = Callback::from(move |_: MouseEvent| {
        let did = did.clone();
        let sync_error_clone = sync_error_clone.clone();
        let request = StorageRequest::TryToSyncRemotely(record_key.clone(), did.clone());
        let storage_agent_for_click = storage_agent_for_click.clone(); // Clone it before use
        let navigator = navigator.clone();
        let cloned_reload_action = cloned_reload_action.clone();

        resync_loading_clone.set(true);
        let resync_loading_clone_for_closure = resync_loading_clone.clone();
        spawn_local(async move {
            let result = storage_agent_for_click.run(request).await;
            match result {
                StorageResponse::Error(err) => {
                    let message_sorry = "Sorry there was an error saving your game. This is still in alpha and has some bugs so please excuse us. If you are logged in with your AT Proto account may try relogging and refreshing this page without hitting new game. It will try to sync again. Sorry again and thanks for trying out at://2048!";
                    alert(message_sorry);
                    log::error!("Error saving game: {:?}", err.to_string());
                }
                RepoError(error) => {
                    log::error!("Error saving game: {:?}", error.to_string());
                    match error {
                        AtRepoSyncError::AuthErrorNeedToReLogin => {
                            match confirm(
                                "Your AT Protocol session has expired. You need to relogin to save your game to your profile. Press confirm to be redirected to login page.",
                            ) {
                                true => {
                                    if let Some(did) = did.as_ref() {
                                        navigator.push(&Route::LoginPageWithDid {
                                            did: did.to_string(),
                                        })
                                    }
                                }
                                false => {
                                    // dispatch.set(UserStore::default());
                                }
                            }
                        }
                        AtRepoSyncError::Error(err) => {
                            sync_error_clone.set(Some(err));
                        }
                        _ => {}
                    }
                }
                StorageResponse::Success => {
                    cloned_reload_action.emit(());
                }
                _ => {}
            };
            resync_loading_clone_for_closure.set(false);
        });
    });

    // let formatted_game_date = js_sys::Date::new(&JsValue::from_str(props.game.created_at.as_str()));
    let formated_date = props
        .game
        .record
        .created_at
        .as_ref()
        .format("%m/%d/%Y %H:%M");
    match validation_result.as_ref() {
        Some(validation_result) => {
            html! {
                <div class="bg-base-100 shadow-lg rounded-lg md:p-6 p-1 flex flex-row">
                    <div class="flex flex-col">
                        <span class="text-md">
                            { format!("Score: {}", validation_result.score) }
                        </span>
                        <MiniGameboard recording={seeded_recording.as_ref().unwrap().clone()} />
                        <span>
                            { match seeded_recording.as_ref() {
                                                Some(recording) => {
                                                    html!{<Link<Route> classes="cursor-pointer underline text-blue-600 visited:text-purple-600" to={Route::SeedPage { seed: recording.seed }}>{ format!("Seed: {}", recording.seed) }</Link<Route>>}
                                                },
                                                None => html!{ <p> {"Loading seed.."} </p> }
                                        } }
                        </span>
                    </div>
                    <div class="pl-2 md:w-3/4 w-1/2 mx-auto">
                        <p>
                            { match seeded_recording.as_ref() {
                                                        Some(recording) => format!("Moves: {}", recording.moves.len().to_string()),
                                                        None => String::from("Loading moves..")
                                                    } }
                        </p>
                        <p>{ formated_date.to_string() }</p>
                        <div class="pt-2">
                            if let Some(_) = props.did.clone() {
                                if props.game.record.sync_status.synced_with_at_repo {
                                    <div class="badge badge-success">
                                        <svg
                                            class="size-[1em]"
                                            xmlns="http://www.w3.org/2000/svg"
                                            viewBox="0 0 24 24"
                                        >
                                            <g
                                                fill="currentColor"
                                                stroke-linejoin="miter"
                                                stroke-linecap="butt"
                                            >
                                                <circle
                                                    cx="12"
                                                    cy="12"
                                                    r="10"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-linecap="square"
                                                    stroke-miterlimit="10"
                                                    stroke-width="2"
                                                />
                                                <polyline
                                                    points="7 13 10 16 17 8"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-linecap="square"
                                                    stroke-miterlimit="10"
                                                    stroke-width="2"
                                                />
                                            </g>
                                        </svg>
                                        { "Synced" }
                                    </div>
                                } else {
                                    if *resync_loading {
                                        <button class="btn btn-outline" disabled=true>
                                            <span class="loading loading-spinner" />
                                            { "loading" }
                                        </button>
                                    } else {
                                        <button onclick={sync_onclick} class="btn btn-outline">
                                            <svg
                                                class="inline-block w-5 fill-[#0a7aff]"
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 640 512"
                                            >
                                                <path
                                                    d="M144 480C64.5 480 0 415.5 0 336c0-62.8 40.2-116.2 96.2-135.9c-.1-2.7-.2-5.4-.2-8.1c0-88.4 71.6-160 160-160c59.3 0 111 32.2 138.7 80.2C409.9 102 428.3 96 448 96c53 0 96 43 96 96c0 12.2-2.3 23.8-6.4 34.6C596 238.4 640 290.1 640 352c0 70.7-57.3 128-128 128l-368 0zm79-217c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l39-39L296 392c0 13.3 10.7 24 24 24s24-10.7 24-24l0-134.1 39 39c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-80-80c-9.4-9.4-24.6-9.4-33.9 0l-80 80z"
                                                />
                                            </svg>
                                            { "Sync" }
                                        </button>
                                    }
                                }
                            }
                        </div>
                        if let Some(err) = sync_error.as_ref() {
                            <span class="text-red-500">{ err }</span>
                        }
                    </div>
                </div>
            }
        }
        None => html! {
            <div class="bg-base-100 shadow-lg rounded-lg md:p-6 p-1">
                <div class="w-full max-w-2xl mx-auto">
                    <span>{ "there was an issue validating this game." }</span>
                </div>
            </div>
        },
    }
}

#[derive(Clone, PartialEq)]
struct PaginationOptions {
    count: u32,
    skip: u32,
    at_proto_cursor: Option<String>,
    /// Set to true once there is no more games to load
    fully_loaded: bool,
}

impl Default for PaginationOptions {
    fn default() -> Self {
        PaginationOptions {
            count: 2,
            skip: 0,
            at_proto_cursor: None,
            fully_loaded: false,
        }
    }
}

async fn get_local_games(
    options: PaginationOptions,
) -> Result<Rc<Vec<Rc<RecordStorageWrapper<game::RecordData>>>>, AtRepoSyncError> {
    log::info!("Getting local games");
    let db = Database::open(DB_NAME)
        .await
        .map_err(|e| AtRepoSyncError::Error(e.to_string()))?;

    let local_games: Vec<RecordStorageWrapper<game::RecordData>> =
        paginated_cursor(db, GAME_STORE, options.count, options.skip)
            .await
            .map_err(|e| AtRepoSyncError::Error(e.to_string()))?;
    Ok(Rc::new(
        local_games
            .iter()
            .map(|game| Rc::new(game.clone()))
            .collect::<Vec<_>>(),
    ))
}

async fn get_games(
    tab_state: &TabState,
    options: PaginationOptions,
) -> Result<Rc<Vec<Rc<RecordStorageWrapper<game::RecordData>>>>, AtRepoSyncError> {
    match tab_state {
        TabState::Local => get_local_games(options).await,
        TabState::Remote => Ok(vec![].into()),
        TabState::Both => {
            //May not implement this
            todo!()
        }
    }
}

#[function_component(HistoryPage)]
pub fn history() -> Html {
    log::info!("History Page rendered");
    let (user_store, _) = use_store::<UserStore>();
    let pagination = use_state(|| PaginationOptions::default());
    let display_games = use_state(|| Rc::new(vec![]));
    let current_tab_state = use_state(|| Rc::new(TabState::Local));

    let display_games_for_mount = display_games.clone();
    let display_games_effect = display_games.clone();
    use_effect_once(move || {
        log::info!("Mounted");
        spawn_local(async move {
            //Can default pagination since this is on load
            match get_local_games(PaginationOptions::default()).await {
                Ok(games) => &display_games_effect.set(games),
                Err(err) => {
                    log::error!("{:?}", err);
                    &()
                }
            };
        });
        || ()
    });

    let pagination_clone = pagination.clone();
    let tab_click_callback = {
        let display_games = display_games_for_mount.clone();
        let pagination_clone = pagination_clone.clone();
        Callback::from(move |tab_state: TabState| {
            pagination_clone.set(PaginationOptions::default());
            let display_games = display_games.clone();
            spawn_local(async move {
                //Just defaulting pagination on tab change
                match get_games(&tab_state, PaginationOptions::default()).await {
                    Ok(games) => &display_games.set(games),
                    Err(err) => {
                        log::error!("{:?}", err);
                        &()
                    }
                };
            })
        })
    };

    let load_more_pagination_clone = pagination_clone.clone();
    let load_more_games_clone = display_games_for_mount.clone();
    let load_more_tab_clone = current_tab_state.clone();
    let load_more_callback = {
        let pagination = load_more_pagination_clone.clone();
        let display_games = load_more_games_clone.clone();
        let current_tab_state = load_more_tab_clone.clone();
        Callback::from(move |_: MouseEvent| {
            let pagination = pagination.clone();
            let display_games = display_games.clone();
            let current_tab_state = current_tab_state.clone();
            let mut new_pagination = PaginationOptions::default();
            new_pagination.skip = pagination.skip + new_pagination.count;
            new_pagination.at_proto_cursor = pagination.at_proto_cursor.clone();
            spawn_local(async move {
                match get_games(current_tab_state.as_ref(), new_pagination.clone()).await {
                    Ok(games) => {
                        let mut combined = (*display_games).to_vec();
                        combined.extend((*games).iter().cloned());
                        display_games.set(Rc::new(combined));
                        new_pagination.fully_loaded = games.len() < new_pagination.count as usize;
                        pagination.set(new_pagination);
                    }
                    Err(err) => {
                        log::error!("{:?}", err);
                    }
                }
            })
        })
    };

    let reload_callback_pagination_clone = pagination_clone.clone();
    let games_reload_callback_clone = display_games_for_mount.clone();
    let current_tab_state_clone = current_tab_state.clone();
    let reload_callback = Callback::from(move |_| {
        let display_games = games_reload_callback_clone.clone();
        let pagination = reload_callback_pagination_clone.clone();
        let current_tab_state = current_tab_state_clone.clone();
        spawn_local(async move {
            match get_games(current_tab_state.as_ref(), (*pagination).clone()).await {
                Ok(games) => &display_games.set(games),
                Err(err) => {
                    log::error!("{:?}", err);
                    &()
                }
            };
        })
    });

    html! {
        <div class="md:p-4 p-1">
            <div class="max-w-4xl mx-auto space-y-6 justify-center">
                <h1 class="text-4xl font-bold text-center md:mb-6 mb-1">{ "Game History" }</h1>
                <div class="bg-base-100 shadow-lg rounded-lg md:p-6 p-1">
                    <div class="w-full max-w-2xl mx-auto">
                        <HistoryTab action={tab_click_callback} />
                        <div class="grid grid-cols-1 gap-6">
                            if display_games_for_mount.as_ref().len() == 0 {
                                <div class="flex items-center justify-center">
                                    <span class="loading loading-spinner loading-lg" />
                                    <h1 class="ml-4 text-3xl font-bold">{ "Loading..." }</h1>
                                </div>
                            } else {
                                { (*display_games_for_mount).iter().enumerate().map(|(i, game)| {
                                    html! {
                                        <GameTile key={i} game={game} did={user_store.did.clone()} reload_action={reload_callback.clone()} />
                                    }
                                }).collect::<Html>() }
                                //TODO figure this logic out
                                if !(*pagination).fully_loaded {
                                    <div class="flex w-full justify-center">
                                        <button onclick={load_more_callback} class="btn btn-outline btn-wide">
                                            { "Load more" }
                                        </button>
                                    </div>
                                }
                            }
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
