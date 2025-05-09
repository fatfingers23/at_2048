use crate::Route;
use crate::idb::{CURRENT_GAME_STORE, DB_NAME, SELF_KEY, transaction_put};
use atrium_api::types::string::Datetime;
use indexed_db_futures::database::Database;
use twothousand_forty_eight::v2::recording::SeededRecording;
use types_2048::blue;
use types_2048::blue::_2048::defs::SyncStatusData;
use web_sys::{HtmlInputElement, InputEvent, SubmitEvent};
use yew::platform::spawn_local;
use yew::{
    Callback, Html, Properties, TargetCast, classes, function_component, html, use_state_eq,
};
use yew_router::hooks::use_navigator;

#[derive(Properties, Clone, PartialEq)]
pub struct SeedProps {
    pub starting_seed: Option<u32>,
}

#[function_component(SeedPage)]
pub fn seed(props: &SeedProps) -> Html {
    let seed_input = use_state_eq(|| props.starting_seed.unwrap_or(0));
    let error = use_state_eq(|| None);
    let navigator = use_navigator().unwrap();
    let on_input_handle = seed_input.clone();
    let error_input = error.clone();

    let oninput = Callback::from(move |input_event: InputEvent| {
        let target: HtmlInputElement = input_event.target_unchecked_into();
        match target.value().parse::<u32>() {
            Ok(seed) => {
                on_input_handle.set(seed);
            }
            Err(_) => {
                error_input.set(Some("Seed must be a number"));
            }
        }
    });
    let error_view_clone = error.clone();
    let onsubmit = {
        let seed_input = seed_input.clone();
        let error_input = error.clone();
        let navigator = navigator.clone();
        Callback::from(move |event: SubmitEvent| {
            let error_callback_clone = error_input.clone();
            error_callback_clone.set(None);
            event.prevent_default();
            let seed_value = *seed_input;
            let error_spawn = error_input.clone();
            let nav = navigator.clone();
            spawn_local(async move {
                let history = SeededRecording::empty(seed_value, 4, 4);
                let history_string: String = (&history).into();

                let db = match Database::open(DB_NAME).await {
                    Ok(db) => db,
                    Err(err) => {
                        panic!("Error opening database: {:?}", err);
                    }
                };
                let current_game = blue::_2048::game::RecordData {
                    completed: false,
                    created_at: Datetime::now(),
                    current_score: 0,
                    seeded_recording: history_string,
                    sync_status: SyncStatusData {
                        created_at: Datetime::now(),
                        hash: "".to_string(),
                        synced_with_at_repo: false,
                        updated_at: Datetime::now(),
                    }
                    .into(),
                    won: false,
                };
                let result = transaction_put(
                    db.clone(),
                    current_game.clone(),
                    CURRENT_GAME_STORE,
                    Some(SELF_KEY.to_string()),
                )
                .await;
                match result {
                    Ok(_) => nav.push(&Route::GamePage),
                    Err(e) => {
                        log::info!("{:?}", current_game);
                        log::error!("{:?}", e.to_string());
                        error_spawn.set(Some("Error creating a new game from that seed"));
                    }
                };
            });
        })
    };

    html! {
        <div class="container mx-auto flex flex-col items-center md:mt-6 mt-4 min-h-screen p-4">
            <h1
                class="md:text-5xl text-4xl font-bold mb-8 bg-gradient-to-r from-primary to-secondary bg-clip-text text-transparent"
            >
                { "at://2048" }
            </h1>
            <div
                class="backdrop-blur-md bg-base-200/50 p-6 rounded-lg shadow-lg mb-8 max-w-md w-full"
            >
                <p class="text-lg mb-4">
                    { "Someone share a starting seed with you? Type it here to replace your current game with that seed and see if you can do better than your friends!" }
                </p>
                <form {onsubmit} class="w-full flex flex-col items-center pt-1">
                    <div class="join w-full">
                        <div class="w-full">
                            <label
                                class={classes!("w-full", "input",  "join-item", error_view_clone.is_none().then(|| Some("dark:input-primary eink:input-neutral")), error_view_clone.is_some().then(|| Some("input-error")))}
                            >
                                <input
                                    {oninput}
                                    value={(*seed_input).to_string()}
                                    type="number"
                                    class="w-full"
                                    placeholder="Enter the game's starting seed here"
                                />
                            </label>
                            if let Some(error_message) = error_view_clone.as_ref() {
                                <div class="text-error">{ error_message }</div>
                            }
                        </div>
                        <button
                            type="submit"
                            class="btn btn-neutral eink:btn-outline dark:btn-primary join-item"
                        >
                            { "New Game" }
                        </button>
                    </div>
                </form>
            </div>
            <div class="container mx-auto p-4" />
        </div>
    }
}
