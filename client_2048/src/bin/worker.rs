use client_2048::agent::{Postcard, StorageTask};
use yew_agent::Registrable;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    StorageTask::registrar().encoding::<Postcard>().register();
}
