use identity_registry_interactor::identity_registry_cli;
use multiversx_sc_snippets::imports::*;

#[tokio::main]
async fn main() {
    identity_registry_cli().await;
}
