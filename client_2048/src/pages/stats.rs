use crate::at_repo_sync::AtRepoSync;
use crate::store::UserStore;
use atrium_api::agent::Agent;
use js_sys::encode_uri_component;
use numfmt::{Formatter, Precision};
use yew::platform::spawn_local;
use yew::{Html, Properties, function_component, html, use_effect_with, use_state};
use yewdux::prelude::*;

#[derive(Properties, PartialEq)]
pub struct BSkyButtonProps {
    pub text: String,
}

#[function_component(BSkyButton)]
pub fn bsky_button(props: &BSkyButtonProps) -> Html {
    let display_text = format!(
        "{}\nThink you can do better? Join in on the fun with @2048.blue.",
        props.text
    );

    let redirect_url = format!(
        "https://bsky.app/intent/compose?text={}",
        encode_uri_component(&display_text)
    );
    html!(
        // <div class="stat-actions">
        <a class="btn btn-sm btn-accent" href={redirect_url}>
            { "Share" }
            <svg
                class="inline-block w-8 fill-[#0a7aff]"
                viewBox="0 0 1024 1024"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
            >
                <path
                    d="M351.121 315.106C416.241 363.994 486.281 463.123 512 516.315C537.719 463.123 607.759 363.994 672.879 315.106C719.866 279.83 796 252.536 796 339.388C796 356.734 786.055 485.101 780.222 505.943C759.947 578.396 686.067 596.876 620.347 585.691C735.222 605.242 764.444 670.002 701.333 734.762C581.473 857.754 529.061 703.903 515.631 664.481C513.169 657.254 512.017 653.873 512 656.748C511.983 653.873 510.831 657.254 508.369 664.481C494.939 703.903 442.527 857.754 322.667 734.762C259.556 670.002 288.778 605.242 403.653 585.691C337.933 596.876 264.053 578.396 243.778 505.943C237.945 485.101 228 356.734 228 339.388C228 252.536 304.134 279.83 351.121 315.106Z"
                />
            </svg>
        </a>
    )
}

#[function_component(StatsPage)]
pub fn stats() -> Html {
    let (user_store, _) = use_store::<UserStore>();
    let stats_state = use_state(|| None);
    let number_formatter = Formatter::new()
        .precision(Precision::Decimals(0))
        .separator(',')
        .expect("Could not build the number formatter.");
    let user_store_clone = user_store.clone();

    use_effect_with(stats_state.clone(), move |stats_state| {
        let stats_state = stats_state.clone();
        spawn_local(async move {
            match user_store_clone.did.clone() {
                None => {
                    let at_repo_sync = AtRepoSync::new_local_repo();

                    match at_repo_sync.get_local_player_stats().await.unwrap_or(None) {
                        Some(stats) => stats_state.set(Some(stats)),
                        _ => {
                            //If there is not a local one create a new stats
                            match at_repo_sync.create_a_new_player_stats().await {
                                Ok(stats) => stats_state.set(Some(stats)),
                                Err(err) => {
                                    log::error!(
                                        "Error creating a new local only stats: {:?}",
                                        err.to_string()
                                    );
                                }
                            }
                        }
                    }
                }
                Some(did) => {
                    let oauth_client = crate::oauth_client::oauth_client().await;
                    let session = match oauth_client.restore(&did).await {
                        Ok(session) => session,
                        Err(err) => {
                            log::error!("{:?}", err);
                            return;
                        }
                    };
                    let agent = Agent::new(session);
                    let at_repo_sync = AtRepoSync::new_logged_in_repo(agent, did);
                    match at_repo_sync.sync_stats().await {
                        Ok(_) => match at_repo_sync.get_local_player_stats().await {
                            Ok(stats) => stats_state.set(stats),
                            Err(err) => {
                                log::error!(
                                    "Error getting local stats after syncing: {:?}",
                                    err.to_string()
                                );
                            }
                        },
                        Err(err) => {
                            log::error!("Error syncing stats: {:?}", err.to_string());
                        }
                    }
                }
            }
        });

        || ()
    });

    if let Some(stats_state) = (*stats_state).clone() {
        //HACK I am very sorry to who ever finds this. I don't have an explanation other than I gave up. Will comeback later...
        let mut formatter = number_formatter.clone();
        let high_score_formatted = formatter.fmt2(stats_state.highest_score.clone());

        let mut formatter = number_formatter.clone();
        let average_score_formatted = formatter.fmt2(stats_state.average_score.clone());

        let mut formatter = number_formatter.clone();
        let total_score_formatted = formatter.fmt2(stats_state.total_score);

        let mut formatter = number_formatter.clone();
        let highest_number_block_formatted = formatter.fmt2(stats_state.highest_number_block);

        let mut formatter = number_formatter.clone();
        let times_twenty_forty_eight_been_found_formatted =
            formatter.fmt2(stats_state.times_twenty_forty_eight_been_found);

        let mut formatter = number_formatter.clone();
        let lowest_turns_till_2048_formatted =
            formatter.fmt2(stats_state.least_moves_to_find_twenty_forty_eight);

        let mut formatter = number_formatter.clone();
        let total_games_formatted = formatter.fmt2(stats_state.games_played);

        html! {
            <div class="min-h-screen bg-base-200 p-4">
                <div class="max-w-4xl mx-auto space-y-4">
                    // Header
                    <div class="card bg-base-100 shadow-xl">
                        <div class="card-body">
                            <h2 class="card-title text-3xl font-bold">
                                { "Your at://2048 Stats" }
                            </h2>
                            <p class="text-base-content/70">
                                { "Track your progress and achievements" }
                            </p>
                        </div>
                    </div>
                    // Main Stats Grid
                    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                        // Score Stats Card
                        <div class="card shadow-xl">
                            <div class="card-body">
                                <h3 class="card-title">{ "Score Statistics" }</h3>
                                <div class="stats stats-vertical shadow">
                                    <div class="stat">
                                        <div class="stat-figure">
                                            <svg
                                                class="inline-block w-11 fill-primary"
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 576 512"
                                            >
                                                <path
                                                    d="M400 0L176 0c-26.5 0-48.1 21.8-47.1 48.2c.2 5.3 .4 10.6 .7 15.8L24 64C10.7 64 0 74.7 0 88c0 92.6 33.5 157 78.5 200.7c44.3 43.1 98.3 64.8 138.1 75.8c23.4 6.5 39.4 26 39.4 45.6c0 20.9-17 37.9-37.9 37.9L192 448c-17.7 0-32 14.3-32 32s14.3 32 32 32l192 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-26.1 0C337 448 320 431 320 410.1c0-19.6 15.9-39.2 39.4-45.6c39.9-11 93.9-32.7 138.2-75.8C542.5 245 576 180.6 576 88c0-13.3-10.7-24-24-24L446.4 64c.3-5.2 .5-10.4 .7-15.8C448.1 21.8 426.5 0 400 0zM48.9 112l84.4 0c9.1 90.1 29.2 150.3 51.9 190.6c-24.9-11-50.8-26.5-73.2-48.3c-32-31.1-58-76-63-142.3zM464.1 254.3c-22.4 21.8-48.3 37.3-73.2 48.3c22.7-40.3 42.8-100.5 51.9-190.6l84.4 0c-5.1 66.3-31.1 111.2-63 142.3z"
                                                />
                                            </svg>
                                        </div>
                                        <div class="stat-title">{ "Highest Score" }</div>
                                        <div class="stat-value">{ high_score_formatted }</div>
                                    </div>
                                    <div class="stat">
                                        <div class="stat-figure ">
                                            <svg
                                                class="inline-block h-9 w-12 fill-primary"
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 576 512"
                                            >
                                                <path
                                                    d="M384 32l128 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L398.4 96c-5.2 25.8-22.9 47.1-46.4 57.3L352 448l160 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-192 0-192 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l160 0 0-294.7c-23.5-10.3-41.2-31.6-46.4-57.3L128 96c-17.7 0-32-14.3-32-32s14.3-32 32-32l128 0c14.6-19.4 37.8-32 64-32s49.4 12.6 64 32zm55.6 288l144.9 0L512 195.8 439.6 320zM512 416c-62.9 0-115.2-34-126-78.9c-2.6-11 1-22.3 6.7-32.1l95.2-163.2c5-8.6 14.2-13.8 24.1-13.8s19.1 5.3 24.1 13.8l95.2 163.2c5.7 9.8 9.3 21.1 6.7 32.1C627.2 382 574.9 416 512 416zM126.8 195.8L54.4 320l144.9 0L126.8 195.8zM.9 337.1c-2.6-11 1-22.3 6.7-32.1l95.2-163.2c5-8.6 14.2-13.8 24.1-13.8s19.1 5.3 24.1 13.8l95.2 163.2c5.7 9.8 9.3 21.1 6.7 32.1C242 382 189.7 416 126.8 416S11.7 382 .9 337.1z"
                                                />
                                            </svg>
                                        </div>
                                        <div class="stat-title">{ "Average Score" }</div>
                                        <div class="stat-value">{ average_score_formatted }</div>
                                    </div>
                                    <div class="stat">
                                        <div class="stat-figure ">
                                            <svg
                                                class="inline-block h-8 w-8 fill-primary"
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 576 512"
                                            >
                                                <path
                                                    d="M160 80c0-26.5 21.5-48 48-48l32 0c26.5 0 48 21.5 48 48l0 352c0 26.5-21.5 48-48 48l-32 0c-26.5 0-48-21.5-48-48l0-352zM0 272c0-26.5 21.5-48 48-48l32 0c26.5 0 48 21.5 48 48l0 160c0 26.5-21.5 48-48 48l-32 0c-26.5 0-48-21.5-48-48L0 272zM368 96l32 0c26.5 0 48 21.5 48 48l0 288c0 26.5-21.5 48-48 48l-32 0c-26.5 0-48-21.5-48-48l0-288c0-26.5 21.5-48 48-48z"
                                                />
                                            </svg>
                                        </div>
                                        <div class="stat-title">{ "Total Score" }</div>
                                        <div class="stat-value">{ total_score_formatted }</div>
                                    </div>
                                </div>
                            </div>
                            <BSkyButton
                                text={format!("High Score: {}\nAverage Score: {}\nTotal Score: {}\n",high_score_formatted, average_score_formatted, total_score_formatted);}
                            />
                        </div>
                        // Achievement Stats Card
                        <div class="card  shadow-xl">
                            <div class="card-body">
                                <h3 class="card-title">{ "Achievements" }</h3>
                                <div class="stats stats-vertical shadow">
                                    <div class="stat">
                                        <div class="stat-figure ">
                                            <svg
                                                class="inline-block h-8 w-8 fill-primary"
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 576 512"
                                            >
                                                <path
                                                    d="M0 96C0 60.7 28.7 32 64 32H384c35.3 0 64 28.7 64 64V416c0 35.3-28.7 64-64 64H64c-35.3 0-64-28.7-64-64V96z"
                                                />
                                            </svg>
                                        </div>
                                        <div class="stat-title">{ "Highest Block" }</div>
                                        <div class="stat-value">
                                            { highest_number_block_formatted }
                                        </div>
                                    </div>
                                    <div class="stat">
                                        <div class="stat-figure ">
                                            <h1 class="text-primary text-xl font-bold">
                                                { "2048" }
                                            </h1>
                                        </div>
                                        <div class="stat-title">{ "Times 2048 Found" }</div>
                                        <div class="stat-value">
                                            { times_twenty_forty_eight_been_found_formatted }
                                        </div>
                                    </div>
                                    <div class="stat">
                                        <div class="stat-figure">
                                            <svg
                                                class="inline-block h-8 w-8 fill-primary"
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 576 512"
                                            >
                                                <path
                                                    d="M320 48a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM125.7 175.5c9.9-9.9 23.4-15.5 37.5-15.5c1.9 0 3.8 .1 5.6 .3L137.6 254c-9.3 28 1.7 58.8 26.8 74.5l86.2 53.9-25.4 88.8c-4.9 17 5 34.7 22 39.6s34.7-5 39.6-22l28.7-100.4c5.9-20.6-2.6-42.6-20.7-53.9L238 299l30.9-82.4 5.1 12.3C289 264.7 323.9 288 362.7 288l21.3 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-21.3 0c-12.9 0-24.6-7.8-29.5-19.7l-6.3-15c-14.6-35.1-44.1-61.9-80.5-73.1l-48.7-15c-11.1-3.4-22.7-5.2-34.4-5.2c-31 0-60.8 12.3-82.7 34.3L57.4 153.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0l23.1-23.1zM91.2 352L32 352c-17.7 0-32 14.3-32 32s14.3 32 32 32l69.6 0c19 0 36.2-11.2 43.9-28.5L157 361.6l-9.5-6c-17.5-10.9-30.5-26.8-37.9-44.9L91.2 352z"
                                                />
                                            </svg>
                                        </div>
                                        <div class="stat-title">{ "Lowest turns to 2048" }</div>
                                        <div class="stat-value">
                                            { lowest_turns_till_2048_formatted }
                                        </div>
                                        <div class="stat-desc">{ "moves" }</div>
                                    </div>
                                </div>
                            </div>
                            <BSkyButton
                                text={format!("Highest Block: {}\nTimes 2048 found: {}\nLowest turns to 2048: {}\n",highest_number_block_formatted, times_twenty_forty_eight_been_found_formatted, lowest_turns_till_2048_formatted);}
                            />
                        </div>
                        // Game History Card
                        <div class="card shadow-xl">
                            <div class="card-body">
                                <h3 class="card-title">{ "Game History" }</h3>
                                <div class="stats stats-vertical shadow">
                                    <div class="stat">
                                        <div class="stat-figure">
                                            <svg
                                                class="inline-block h-8 w-12 fill-primary"
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 576 512"
                                            >
                                                <path
                                                    d="M192 64C86 64 0 150 0 256S86 448 192 448l256 0c106 0 192-86 192-192s-86-192-192-192L192 64zM496 168a40 40 0 1 1 0 80 40 40 0 1 1 0-80zM392 304a40 40 0 1 1 80 0 40 40 0 1 1 -80 0zM168 200c0-13.3 10.7-24 24-24s24 10.7 24 24l0 32 32 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-32 0 0 32c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-32-32 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l32 0 0-32z"
                                                />
                                            </svg>
                                        </div>
                                        <div class="stat-title">{ "Total Games" }</div>
                                        <div class="stat-value">{ total_games_formatted }</div>
                                    </div>
                                    <div class="stat">
                                        <div class="stat-title">{ "Win Rate" }</div>
                                        <div class="stat-value">
                                            { format!("{}%",
                                                        if stats_state.games_played > 0 {
                                                            (stats_state.times_twenty_forty_eight_been_found as f64
                                                             / stats_state.games_played as f64
                                                             * 100.0).round()
                                                        } else {
                                                            0.0
                                                        }
                                                    ) }
                                        </div>
                                    </div>
                                    // <div class="stat">
                                    //     <div class="stat-title">{ "First Game" }</div>
                                    //     <div class="stat-desc">
                                    //         { stats_state.created_at.as_str() }
                                    //     </div>
                                    // </div>
                                </div>
                            </div>
                            <BSkyButton
                                text={format!("I've played {} games of at://2048", total_games_formatted);}
                            />
                        </div>
                    </div>
                </div>
            </div>
        }
    } else {
        html! {
            <div class="flex flex-col items-center justify-center h-screen bg-base-200">
                <div class="flex items-center justify-center">
                    <span class="loading loading-spinner loading-lg" />
                    <h1 class="ml-4 text-3xl font-bold">{ "Loading Stats..." }</h1>
                </div>
            </div>
        }
    }
}
