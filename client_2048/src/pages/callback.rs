use crate::Route;
use crate::at_repo_sync::AtRepoSync;
use crate::oauth_client::oauth_client;
use crate::store::UserStore;
use atrium_api::agent::Agent;
use atrium_oauth::CallbackParams;
use yew::platform::spawn_local;
use yew::{Html, function_component, html, use_state_eq};
use yew_hooks::use_effect_once;
use yew_router::hooks::use_location;
use yew_router::prelude::use_navigator;
use yewdux::prelude::*;

#[function_component(CallbackPage)]
pub fn callback() -> Html {
    log::info!("Callback rendered");
    let location = use_location();
    let oauth_client = oauth_client();
    let (user_store, dispatch) = use_store::<UserStore>();
    let error = use_state_eq(|| None);
    let navigator = use_navigator().unwrap();
    let error_view_clone = error.clone();

    use_effect_once(move || {
        match location {
            None => {}
            Some(location) => {
                spawn_local(async move {
                    log::info!("Callback effect called");
                    match serde_html_form::from_str::<CallbackParams>(
                        &*location.query_str().replace("?", ""),
                    ) {
                        Ok(params) => {
                            match oauth_client.await.callback(params).await {
                                Ok((session, _)) => {
                                    let agent = Agent::new(session);
                                    let did = agent.did().await.unwrap();

                                    let profile = agent
                                    .api
                                    .app
                                    .bsky
                                    .actor
                                    .get_profile(
                                        atrium_api::app::bsky::actor::get_profile::ParametersData {
                                            actor: atrium_api::types::string::AtIdentifier::Did(did.clone()),
                                        }
                                            .into(),
                                    )
                                    .await;
                                    //HACK
                                    dispatch.set(UserStore {
                                        did: Some(did.clone()),
                                        handle: match profile {
                                            Ok(profile) => Some(profile.handle.clone()),
                                            Err(_) => None,
                                        },
                                    });
                                    let at_repo_sync = AtRepoSync::new_logged_in_repo(agent, did);
                                    match at_repo_sync.sync_profiles().await {
                                        Ok(_) => {}
                                        Err(err) => {
                                            log::error!("Error: {:?}", err.to_string());
                                            error_view_clone.set(Some("There was an error with your login and syncing your profiles. Try again or can check the console for more details."));
                                        }
                                    }

                                    match at_repo_sync.sync_stats().await {
                                        Ok(_) => {}
                                        Err(err) => {
                                            log::error!("Error: {:?}", err.to_string());
                                            error_view_clone.set(Some("There was an error with your login and syncing your stats. Try again or can check the console for more details."));
                                        }
                                    }

                                    navigator.push(&Route::GamePage)
                                } // None => {
                                //     error_view_clone.set(Some("There was an error with your login. Try again or can check the console for more details."));
                                // }
                                Err(err) => {
                                    log::error!("Error: {:?}", err);
                                    error_view_clone.set(Some("There was an error with your login. Try again or can check the console for more details."));
                                }
                            }
                        }
                        Err(_) => {
                            error_view_clone.set(Some("No call back parameters found in the URL."));
                        }
                    }
                });
            }
        };
        || ()
    });
    html! {
        <div class="container mx-auto flex flex-col items-center md:mt-6 mt-4 min-h-screen p-4">
            if user_store.did.is_none() && error.as_ref().is_none() {
                <div class="flex items-center justify-center">
                    <span class="loading loading-spinner loading-lg" />
                    <h1 class="ml-4 text-3xl font-bold">{ "Loading..." }</h1>
                </div>
            }
            if let Some(error) = error.as_ref() {
                <h1 class="text-4xl">{ error }</h1>
            }
        </div>
    }
}
