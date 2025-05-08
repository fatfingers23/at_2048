use implicit_clone::unsync::*;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{HtmlInputElement, KeyboardEvent};
use yew::prelude::*;

pub enum HistoryAction {}

struct HistoryState {
    games: Vec<types_2048::blue::_2048::game::RecordData>,
}

#[function_component(HistoryPage)]
pub fn history() -> Html {
    // let game_history = use_state(IArray::<types_2048::blue::_2048::game::RecordData>::default);
    // let onkeyup = {
    //     let folks = folks.clone();
    //     Callback::from(move |e: KeyboardEvent| {
    //         if e.key() == "Enter" {
    //             let event: Event = e.dyn_into().unwrap_throw();
    //             let event_target = event.target().unwrap_throw();
    //             let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
    //             let name = target.value();
    //             target.set_value("");
    //             let mut vec = folks.to_vec();
    //             vec.push(IString::from(name));
    //             folks.set(IArray::from(vec));
    //         }
    //     })
    // };

    html! {
        <>
            <h1>{ "History" }</h1>
        </>
    }
}
