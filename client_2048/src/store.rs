use atrium_api::types::string::{Did, Handle};
use serde::{Deserialize, Serialize};
use yewdux::prelude::*;

#[derive(Default, PartialEq, Serialize, Deserialize, Store)]
#[store(storage = "local")]
#[derive(Clone)]
pub struct UserStore {
    pub did: Option<Did>,
    pub handle: Option<Handle>,
}

//Incase I need a debug listener later
// #[store(storage = "local", listener(LogListener))]
// struct LogListener;
// impl Listener for LogListener {
//     type Store = UserStore;
//
//     fn on_change(&self, _cx: &Context, state: Rc<Self::Store>) {
//         log::info!("Theres a did {}", state.did.is_some());
//     }
// }
