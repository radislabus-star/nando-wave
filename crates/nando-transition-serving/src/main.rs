use nando_transition_serving::{ServingConfig, serve};

#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("--print-runtime-contract-sha256") {
        println!(
            "{}",
            nando_response_actor::response_runtime_contract_sha256()
        );
        return;
    }
    let config = match ServingConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("nando-transition-serving: {error}");
            std::process::exit(2);
        }
    };
    if let Err(error) = serve(config).await {
        eprintln!("nando-transition-serving: {error}");
        std::process::exit(1);
    }
}
