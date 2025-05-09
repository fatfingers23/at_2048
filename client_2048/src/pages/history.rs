use std::rc::Rc;
use types_2048::blue;
use yew::prelude::*;

struct HistoryState {
    games: Vec<types_2048::blue::_2048::game::RecordData>,
}

pub enum HistoryAction {
    GetLocalGames,
}

impl Reducible for HistoryState {
    /// Reducer Action Type
    type Action = HistoryAction;

    /// Reducer Function
    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let games: Vec<blue::_2048::game::RecordData> = match action {
            HistoryAction::GetLocalGames => {
                // let db = match Database::open(DB_NAME).await {
                //     Ok(db) => db,
                //     Err(err) => {
                //         log::error!("{}", err);
                //         vec![]
                //     }
                // };
                vec![]
            }
        };

        Self { games }.into()
    }
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
        <div class="p-4">
            <div class="max-w-4xl mx-auto space-y-4 justify-center">
                <div
                    class=" bg-base-100 relative flex min-h-[6rem] max-w-4xl min-w-[18rem] flex-wrap items-center justify-center gap-2 overflow-x-hidden bg-cover bg-top p-4 xl:py-10 "
                >
                    <div role="tablist" class="tabs tabs-lift">
                        <a role="tab" class="tab">{ "Local" }</a>
                        <a role="tab" class="tab tab-active">{ "Remote" }</a>
                        <a role="tab" class="tab">{ "Both" }</a>
                    </div>
                </div>
                // Header
                // <div class="card bg-base-100 shadow-xl">
                //     <div class="card-body">
                //         <h2 class="card-title text-3xl font-bold">{ "Your at://2048 Stats" }</h2>
                //         <p class="text-base-content/70">
                //             { "Track your progress and achievements" }
                //         </p>
                //     </div>
                // </div>
                // Main Stats Grid
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4" />
            </div>
        </div>
    }
}
