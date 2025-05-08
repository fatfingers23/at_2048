use implicit_clone::unsync::*;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{HtmlInputElement, KeyboardEvent};
use yew::prelude::*;

#[function_component(HistoryPage)]
pub fn history() -> Html {
    let folks = use_state(IArray::<IString>::default);
    let onkeyup = {
        let folks = folks.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let event: Event = e.dyn_into().unwrap_throw();
                let event_target = event.target().unwrap_throw();
                let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
                let name = target.value();
                target.set_value("");
                let mut vec = folks.to_vec();
                vec.push(IString::from(name));
                folks.set(IArray::from(vec));
            }
        })
    };

    html! {
        <>
        <h2>{"Input"}</h2>
        <input {onkeyup} />
        <h2>{"Output"}</h2>
        <ul>
            { for folks.iter().map(|name| html! { <li>{ name.as_str() }</li> }) }
        </ul>
        </>
    }
}
