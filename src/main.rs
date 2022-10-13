fn main() {
    #[cfg(feature = "ui")]
    {
        wasm_logger::init(wasm_logger::Config::default());

        yew::start_app::<hal_sim::App>();
    }
}
