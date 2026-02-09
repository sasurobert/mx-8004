#![allow(non_snake_case)]

pub mod config;
mod reputation_registry_proxy;

use config::Config;
use multiversx_sc_snippets::imports::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

const STATE_FILE: &str = "state.toml";

pub async fn reputation_registry_cli() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let config = Config::new();
    let mut interact = ContractInteract::new(config).await;
    match cmd.as_str() {
        "deploy" => interact.deploy().await,
        "upgrade" => interact.upgrade().await,
        "submit_feedback" => interact.submit_feedback().await,
        "authorize_feedback" => interact.authorize_feedback().await,
        "append_response" => interact.append_response().await,
        "get_reputation_score" => interact.reputation_score().await,
        "get_total_jobs" => interact.total_jobs().await,
        "get_validation_contract_address" => interact.validation_contract_address().await,
        "get_identity_contract_address" => interact.identity_contract_address().await,
        "has_given_feedback" => interact.has_given_feedback().await,
        "is_feedback_authorized" => interact.is_feedback_authorized().await,
        "get_agent_response" => interact.agent_response().await,
        "set_identity_contract_address" => interact.set_identity_contract_address().await,
        "set_validation_contract_address" => interact.set_validation_contract_address().await,
        _ => panic!("unknown command: {}", &cmd),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    contract_address: Option<Bech32Address>
}

impl State {
        // Deserializes state from file
        pub fn load_state() -> Self {
            if Path::new(STATE_FILE).exists() {
                let mut file = std::fs::File::open(STATE_FILE).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content).unwrap();
                toml::from_str(&content).unwrap()
            } else {
                Self::default()
            }
        }
    
        /// Sets the contract address
        pub fn set_address(&mut self, address: Bech32Address) {
            self.contract_address = Some(address);
        }
    
        /// Returns the contract address
        pub fn current_address(&self) -> &Bech32Address {
            self.contract_address
                .as_ref()
                .expect("no known contract, deploy first")
        }
    }
    
    impl Drop for State {
        // Serializes state to file
        fn drop(&mut self) {
            let mut file = std::fs::File::create(STATE_FILE).unwrap();
            file.write_all(toml::to_string(self).unwrap().as_bytes())
                .unwrap();
        }
    }

pub struct ContractInteract {
    interactor: Interactor,
    wallet_address: Address,
    contract_code: BytesValue,
    state: State
}

impl ContractInteract {
    pub async fn new(config: Config) -> Self {
        let mut interactor = Interactor::new(config.gateway_uri())
            .await
            .use_chain_simulator(config.use_chain_simulator());

        interactor.set_current_dir_from_workspace("reputation-registry");
        let wallet_address = interactor.register_wallet(test_wallets::alice()).await;

        // Useful in the chain simulator setting
        // generate blocks until ESDTSystemSCAddress is enabled
        interactor.generate_blocks_until_all_activations().await;
        
        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/reputation-registry.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            wallet_address,
            contract_code,
            state: State::load_state()
        }
    }

    pub async fn deploy(&mut self) {
        let validation_contract_address = ManagedAddress::<StaticApi>::zero();
        let identity_contract_address = ManagedAddress::<StaticApi>::zero();

        let new_address = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(30_000_000u64)
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .init(validation_contract_address, identity_contract_address)
            .code(&self.contract_code)
            .returns(ReturnsNewAddress)
            .run()
            .await;
        let new_address_bech32 = new_address.to_bech32_default();
        println!("new address: {new_address_bech32}");
        self.state.set_address(new_address_bech32);
    }

    pub async fn upgrade(&mut self) {
        let response = self
            .interactor
            .tx()
            .to(self.state.current_address())
            .from(&self.wallet_address)
            .gas(30_000_000u64)
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .upgrade()
            .code(&self.contract_code)
            .code_metadata(CodeMetadata::UPGRADEABLE)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn submit_feedback(&mut self) {
        let job_id = ManagedBuffer::new_from_bytes(&b""[..]);
        let agent_nonce = 0u64;
        let rating = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .submit_feedback(job_id, agent_nonce, rating)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn authorize_feedback(&mut self) {
        let job_id = ManagedBuffer::new_from_bytes(&b""[..]);
        let client = ManagedAddress::<StaticApi>::zero();

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .authorize_feedback(job_id, client)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn append_response(&mut self) {
        let job_id = ManagedBuffer::new_from_bytes(&b""[..]);
        let response_uri = ManagedBuffer::new_from_bytes(&b""[..]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .append_response(job_id, response_uri)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn reputation_score(&mut self) {
        let agent_nonce = 0u64;

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .reputation_score(agent_nonce)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn total_jobs(&mut self) {
        let agent_nonce = 0u64;

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .total_jobs(agent_nonce)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn validation_contract_address(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .validation_contract_address()
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn identity_contract_address(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .identity_contract_address()
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn has_given_feedback(&mut self) {
        let job_id = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .has_given_feedback(job_id)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn is_feedback_authorized(&mut self) {
        let job_id = ManagedBuffer::new_from_bytes(&b""[..]);
        let client = ManagedAddress::<StaticApi>::zero();

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .is_feedback_authorized(job_id, client)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn agent_response(&mut self) {
        let job_id = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .agent_response(job_id)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn set_identity_contract_address(&mut self) {
        let address = ManagedAddress::<StaticApi>::zero();

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .set_identity_contract_address(address)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn set_validation_contract_address(&mut self) {
        let address = ManagedAddress::<StaticApi>::zero();

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(reputation_registry_proxy::ReputationRegistryProxy)
            .set_validation_contract_address(address)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

}
