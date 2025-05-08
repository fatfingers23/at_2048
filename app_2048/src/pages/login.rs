use crate::oauth_client::{handle_resolve_from_did, oauth_client};
use crate::store::UserStore;
use atrium_api::types::string::Did;
use atrium_oauth::{AuthorizeOptions, KnownScope, Scope};
use std::str::FromStr;
use web_sys::{HtmlInputElement, InputEvent, SubmitEvent};
use yew::platform::spawn_local;
use yew::{
    Callback, Html, Properties, TargetCast, classes, function_component, html, use_state_eq,
};
use yew_hooks::use_effect_once;
use yewdux::use_store;

pub async fn redirect_to_auth(handle: String) -> Result<(), String> {
    let client = oauth_client().await;
    let oauth_client = client.clone();

    let url = oauth_client
        .authorize(
            handle.to_string().to_lowercase(),
            AuthorizeOptions {
                scopes: vec![
                    Scope::Known(KnownScope::Atproto),
                    Scope::Known(KnownScope::TransitionGeneric),
                ],
                ..Default::default()
            },
        )
        .await;
    match url {
        Ok(url) => {
            let window = gloo_utils::window();

            match window.location().set_href(&url) {
                Ok(_) => Ok(()),
                Err(err) => {
                    log::error!("login error: {:?}", err);
                    Err(String::from("Error redirecting to the login page"))
                }
            }
        }
        Err(err) => {
            log::error!("login error: {}", err);
            let error_str = format!("login error: {}", err);
            Err(error_str)
        }
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct LoginProps {
    pub did: Option<String>,
}

#[function_component(LoginPage)]
pub fn login(props: &LoginProps) -> Html {
    let props = props.clone();
    let handle = use_state_eq(|| String::new());
    let has_redirect_did = props.did.is_some();

    let starting_error = if has_redirect_did {
        Some(
            "Your login session has expired. Attempting to redirect you to the login page."
                .to_string(),
        )
    } else {
        None
    };
    let error_state = use_state_eq(|| starting_error);
    let error_state_effect = error_state.clone();
    let (_, dispatch) = use_store::<UserStore>();

    use_effect_once(move || {
        if let Some(users_did) = props.did.clone() {
            spawn_local(async move {
                let error = match Did::from_str(&users_did) {
                    Ok(did) => match handle_resolve_from_did(did).await {
                        Some(handle) => match redirect_to_auth(handle).await {
                            Ok(_) => None,
                            Err(err) => Some(err),
                        },
                        None => None,
                    },
                    Err(err) => Some(err.to_string()),
                };
                if let Some(error) = error {
                    dispatch.set(UserStore::default());
                    error_state_effect.set(Some(format!("Error from redirect: {}", error)));
                }
            });
        }
        || ()
    });

    let on_input_handle = handle.clone();
    let oninput = Callback::from(move |input_event: InputEvent| {
        let target: HtmlInputElement = input_event.target_unchecked_into();
        on_input_handle.set(target.value());
    });
    let error_view_clone = error_state.clone();
    let onsubmit = {
        move |event: SubmitEvent| {
            let error_callback_clone = error_state.clone();
            error_callback_clone.set(None);
            event.prevent_default();
            let handle = handle.clone();
            spawn_local(async move {
                match redirect_to_auth((*handle).clone()).await {
                    Ok(_) => return,
                    Err(err) => {
                        error_callback_clone.set(Some(err));
                        return;
                    }
                }
            });
        }
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
                    { "You can use at://2048 without a login. But if you do login with your ATProto account you can:" }
                </p>
                <ul class="list-disc list-inside space-y-2 mb-4">
                    <li>{ "Save your progress across multiple devices" }</li>
                    <li>{ "Track your statistics across multiple devices" }</li>
                    <li>{ "Compete on global leaderboards (future)" }</li>
                    <li>{ "See friends scores (future)" }</li>
                    <li>{ "The data is 100% yours stored in your PDS" }</li>
                </ul>
                <form {onsubmit} class="w-full flex flex-col items-center pt-1">
                    <div class="join w-full">
                        <div class="w-full">
                            <label
                                class={classes!("w-full", "input",  "join-item", error_view_clone.is_none().then(|| Some("dark:input-primary eink:input-neutral")), error_view_clone.is_some().then(|| Some("input-error")))}
                            >
                                <input
                                    {oninput}
                                    type="text"
                                    class="w-full"
                                    placeholder="Enter your handle (eg 2048.bsky.social)"
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
                            { "Login" }
                        </button>
                    </div>
                </form>
            </div>
            <div class="container mx-auto p-4" />
        </div>
    }
}
