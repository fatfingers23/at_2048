use crate::at_repo_sync::AtRepoSyncError;
use crate::idb::{DB_NAME, GAME_STORE, RecordStorageWrapper, paginated_cursor};
use crate::store::UserStore;
use atrium_api::types::string::{Datetime, Did};
use indexed_db_futures::database::Database;
use std::rc::Rc;
use types_2048::blue::_2048::defs::SyncStatusData;
use types_2048::blue::_2048::game;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;
use yew::platform::spawn_local;
use yew::prelude::*;
use yew_hooks::{use_async, use_async_with_options};
use yewdux::context_provider::Props;
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
                onclick={onclick.clone()}
                role="tab"
                class={classes!("tab", (*tab_state == TabState::Remote).then(|| Some("tab-active")))}
            >
                { "Remote" }
            </a>
            <a
                {onclick}
                role="tab"
                class={classes!("tab", (*tab_state == TabState::Both).then(|| Some("tab-active")))}
            >
                { "Both" }
            </a>
        </div>
    }
}

#[derive(Properties, Clone, PartialEq)]
struct GameTileProps {
    game: Rc<game::RecordData>,
}

#[function_component(GameTile)]
fn game_tile(props: &GameTileProps) -> Html {
    // <div class="bg-base-200 p-4 rounded-lg shadow">
    //     // <h3>{format!("Game {}", game)}</h3>
    //     <p>{format!("Score: {}", game.current_score)}</p>
    //     // <p>{format!("Date: {}", chrono::NaiveDateTime::from_timestamp_opt(game.timestamp, 0).unwrap().format("%Y-%m-%d"))}</p>
    //     </div>
    html! {
        <div class="bg-base-100 shadow-lg rounded-lg md:p-6 p-1">
            <div class="w-full max-w-2xl mx-auto">
                <h3>{ "Game 1" }</h3>
                <p>{ format!("Score: {}", props.game.current_score) }</p>
                <p>{ format!("Date: {}", props.game.created_at.as_str()) }</p>
        </div>
        </div>
    }
}

#[function_component(HistoryPage)]
pub fn history() -> Html {
    log::info!("Ribbit");
    let (user_store, _) = use_store::<UserStore>();
    let place_holder_games = vec![Rc::new(game::RecordData {
        completed: false,
        created_at: Datetime::now(),
        current_score: 0,
        seeded_recording: "".to_string(),
        sync_status: SyncStatusData {
            created_at: Datetime::now(),
            hash: "".to_string(),
            synced_with_at_repo: false,
            updated_at: Datetime::now(),
        }
        .into(),
        won: false,
    })];
    let display_games = use_state(|| Rc::new(place_holder_games));

    let at_repo_sync = match &user_store.did {
        Some(did) => {
            // let oauth_client = crate::oauth_client::oauth_client();
            // let session = match oauth_client.restore(&did).await {
            //     Ok(session) => session,
            //     Err(err) => {
            //         log::error!("{:?}", err);
            //         return;
            //     }
            // };
            // let agent = Agent::new(session);
            // let at_repo_sync = AtRepoSync::new_logged_in_repo(agent, did);
        }
        None => {}
    };

    let tab_click_callback = {
        let display_games = display_games.clone();
        Callback::from(move |tab_state: TabState| {
            let display_games = display_games.clone();
            match tab_state {
                //TODO do async stuff in here with spawn locla and set outside state?
                TabState::Local => spawn_local(async move {
                    let db = Database::open(DB_NAME)
                        .await
                        .map_err(|e| AtRepoSyncError::Error(e.to_string()))
                        .unwrap();
                    let local_games: Vec<RecordStorageWrapper<game::RecordData>> =
                        paginated_cursor(db, GAME_STORE, 10, 0).await.unwrap();
                    log::info!("{:?}", local_games);
                    display_games.set(Rc::new(
                        local_games
                            .into_iter()
                            .map(|wrapper| Rc::new(wrapper.record))
                            .collect(),
                    ));
                }),
                TabState::Remote => {}
                TabState::Both => {}
            }
        })
    };
    // let local_games = use_async(async move {});
    html! {
        <div class="md:p-4 p-1">
            <div class="max-w-4xl mx-auto space-y-6 justify-center">
                <h1 class="text-4xl font-bold text-center md:mb-6 mb-1">{ "Game History" }</h1>
                <div class="bg-base-100 shadow-lg rounded-lg md:p-6 p-1">
                    <div class="w-full max-w-2xl mx-auto">
                        <HistoryTab action={tab_click_callback} />
                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                            { (*display_games).iter().enumerate().map(|(i, game)| {
                                html! {
                                    <GameTile key={i} game={game} />
                                }
                            }).collect::<Html>() }
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
