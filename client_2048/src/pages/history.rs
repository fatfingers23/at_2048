use crate::at_repo_sync::AtRepoSyncError;
use crate::idb::{DB_NAME, GAME_STORE, RecordStorageWrapper, paginated_cursor};
use crate::pages::game::{Grid, Tile, TileProps};
use crate::pages::seed::_SeedProps::starting_seed;
use crate::store::UserStore;
use atrium_api::types::string::{Datetime, Did};
use indexed_db_futures::database::Database;
use std::rc::Rc;
use twothousand_forty_eight::unified::game::GameState;
use twothousand_forty_eight::unified::validation::{Validatable, ValidationResult};
use twothousand_forty_eight::v2::io::SeededRecordingParseError;
use twothousand_forty_eight::v2::recording::SeededRecording;
use twothousand_forty_eight::v2::replay::MoveReplayError;
use types_2048::blue::_2048::defs::SyncStatusData;
use types_2048::blue::_2048::game;
use types_2048::blue::_2048::game::RecordData;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlElement;
use yew::platform::spawn_local;
use yew::prelude::*;
use yew::props;
use yew_hooks::{use_async, use_async_with_options, use_effect_once, use_mount};
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

#[function_component(MiniTile)]
fn mini_tile(props: &TileProps) -> Html {
    let TileProps {
        tile_value: tile_value_ref,
        new_tile: new_tile_ref,
        x,
        y,
        size,
    } = props;

    let text = if *tile_value_ref == 0 {
        String::new()
    } else {
        tile_value_ref.to_string()
    };

    //TODO fix font size for big numbers
    let tile_class = crate::pages::game::get_bg_color_and_text_color(*tile_value_ref);
    html! {
        <div
            class="  p-1 flex items-center justify-center"
        >
            <div
                class={format!(
                        "flex items-center justify-center w-full h-full {} font-bold text rounded-md",
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

                id="game-board"
                class="flex-1 mx-auto w-full bg-light-board-background shadow-2xl rounded-md p-1"
            >
                    <div class={classes!(String::from("grid grid-cols-4 p-2 w-full h-full"))}>
                        { flatten_tiles.into_iter().map(|tile| {

                                html! { <MiniTile key={tile.id} tile_value={tile.value} new_tile={tile.new} x={tile.x} y={tile.y} size={4} /> }
                            }).collect::<Html>() }
                    </div>

            </div>
    }
}

#[derive(Properties, Clone, PartialEq)]
struct GameTileProps {
    game: Rc<game::RecordData>,
}

#[function_component(GameTile)]
fn game_tile(props: &GameTileProps) -> Html {
    //Validation results
    //Score
    //valid or no

    //Seed recording
    //Seed

    //From prop
    //date
    //won
    //sync status

    let game = props.game.clone();
    let seeded_recording = use_state(|| None);
    let validation_result: UseStateHandle<Option<ValidationResult>> = use_state(|| None);

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

    // let formatted_game_date = js_sys::Date::new(&JsValue::from_str(props.game.created_at.as_str()));
    let formated_date = props.game.created_at.as_ref().format("%m/%d/%Y %H:%M");
    match validation_result.as_ref() {
        Some(validation_result) => {
            html! {
                <div class="bg-base-100 shadow-lg rounded-lg md:p-6 p-1 flex flex-row">
                    <MiniGameboard recording={seeded_recording.as_ref().unwrap().clone()} />
                    <div class="pl-1 w-3/4 mx-auto">
                        <p>{ format!("Score: {}", validation_result.score) }</p>
                        <p>
                            { match seeded_recording.as_ref() {
                                Some(recording) => format!("Seed: {}", recording.seed),
                                None => String::from("Loading seed..")
                            } }
                        </p>
                        <p>
                { match seeded_recording.as_ref() {
                    Some(recording) => recording.moves.len().to_string(),
                    None => "".to_string()
                }}
                        </p>
                        <p>{ format!("Date: {}", formated_date) }</p>
                        // <p>{ format!("Moves: {}", history.moves.len()) }</p>
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

async fn get_local_games() -> Result<Rc<Vec<Rc<game::RecordData>>>, AtRepoSyncError> {
    log::info!("Getting local games");
    let db = Database::open(DB_NAME)
        .await
        .map_err(|e| AtRepoSyncError::Error(e.to_string()))
        .unwrap();
    let local_games: Vec<RecordStorageWrapper<game::RecordData>> =
        paginated_cursor(db, GAME_STORE, 10, 0).await.unwrap();
    Ok(Rc::new(
        local_games
            .into_iter()
            .map(|wrapper| Rc::new(wrapper.record))
            .collect(),
    ))
}

#[function_component(HistoryPage)]
pub fn history() -> Html {
    log::info!("Ribbit");
    let (user_store, _) = use_store::<UserStore>();

    let display_games = use_state(|| Rc::new(vec![]));

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

    let display_games_for_mount = display_games.clone();
    use_effect_once(move || {
        log::info!("Mounted");
        spawn_local(async move {
            match get_local_games().await {
                Ok(games) => &display_games.set(games),
                Err(err) => {
                    log::error!("{:?}", err);
                    &()
                }
            };
        });
        || ()
    });

    let tab_click_callback = {
        let display_games = display_games_for_mount.clone();
        Callback::from(move |tab_state: TabState| {
            let display_games = display_games.clone();
            match tab_state {
                //TODO do async stuff in here with spawn locla and set outside state?
                TabState::Local => spawn_local(async move {
                    match get_local_games().await {
                        Ok(games) => &display_games.set(games),
                        Err(err) => {
                            log::error!("{:?}", err);
                            &()
                        }
                    };
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
                        <div class="grid grid-cols-1  gap-6">
                            { (*display_games_for_mount).iter().enumerate().map(|(i, game)| {
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
