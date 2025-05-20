use gloo::storage::{LocalStorage, Storage};
use web_sys::wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement, HtmlSelectElement};
use yew::prelude::*;
use yew::{Callback, Html, function_component, html, use_state};

#[function_component(ThemePicker)]
pub fn theme_picker() -> Html {
    let themes = vec!["light", "dark", "eink"];
    //Detect browser preferred theme
    let browser_default = match gloo_utils::window().match_media("(prefers-color-scheme: dark)") {
        Ok(result) => match result {
            Some(_) => "dark",
            None => "light",
        },
        Err(_) => {
            log::error!("Error getting browser theme");
            "light"
        }
    };
    let saved_theme = LocalStorage::get("theme").unwrap_or_else(|_| browser_default.to_string());
    let selected_theme = use_state(|| saved_theme);

    let on_change_selected_theme = selected_theme.clone();
    let onchange = Callback::from(move |event: Event| {
        let input: HtmlSelectElement = event.target_unchecked_into();
        let _ = if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let html_root_element = document.get_elements_by_tag_name("html").item(0).unwrap();
                let html_root_element: HtmlElement = html_root_element.dyn_into().unwrap();
                let theme = input.value();
                on_change_selected_theme.set(theme.clone());
                LocalStorage::set("theme", theme.clone().as_str()).unwrap();
                html_root_element
                    .set_attribute("data-theme", theme.as_str())
                    .unwrap();
            }
        };
    });
    let current_theme = selected_theme.clone();
    html! {
        <div class="flex-none">
            <fieldset class="fieldset">
                <legend class="md:hidden sm:block fieldset-legend">{ "Theme" }</legend>
                <select {onchange} class="select">
                    { for themes.iter().map(|theme| html! {
                    if *theme == *current_theme {
                     <option selected=true>{theme.to_string()}</option>
                    }
                    else {
                        <option>{theme.to_string()}</option>
                    }}) }
                </select>
            </fieldset>
        </div>
    }
}
