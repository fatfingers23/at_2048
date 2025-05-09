// use idb::{Database, Error};
use yew::Renderer;
use yew::platform::spawn_local;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    spawn_local(async move {
        let db_create_result = client_2048::idb::create_database().await;
        match db_create_result {
            Ok(_) => {}
            Err(err) => {
                log::error!("{:?}", err);
            }
        }
    });
    Renderer::<client_2048::App>::new().render();
}
