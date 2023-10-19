fn main() {
    #[cfg(feature = "ui")]
    {
        wasm_logger::init(wasm_logger::Config::default());

        yew::Renderer::<hal_sim::ui::App>::new().render();
    }
}
