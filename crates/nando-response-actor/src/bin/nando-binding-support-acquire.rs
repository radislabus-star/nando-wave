#[path = "binding_support_acquire/app.rs"]
mod app;

fn main() -> Result<(), String> {
    app::main()
}
