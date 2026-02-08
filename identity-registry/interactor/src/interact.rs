#![allow(non_snake_case)]

pub mod config;
pub mod identity_registry_proxy;

use crate::identity_registry_proxy::MetadataEntry;
use config::Config;
use multiversx_sc_snippets::imports::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

const STATE_FILE: &str = "state.toml";

pub async fn identity_registry_cli() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let config = Config::new();
    let mut interact = ContractInteract::new(config).await;
    match cmd.as_str() {
        "deploy" => interact.deploy().await,
        "upgrade" => interact.upgrade().await,
        "issue_token" => interact.issue_token().await,
        "register_agent" => interact.register_agent().await,
        "update_agent" => interact.update_agent().await,
        "set_metadata" => interact.set_metadata().await,
        "get_metadata" => interact.get_metadata().await,
        "get_agent_service_config" => interact.get_agent_service_config().await,
        "get_agent" => interact.get_agent().await,
        "get_agent_id" => interact.get_agent_id().await,
        "get_agent_token_id" => interact.agent_token_id().await,
        "agent_token_nonce" => interact.agent_token_nonce().await,
        "get_agent_owner" => interact.agent_owner().await,
        "get_agent_service_price" => interact.agent_service_price().await,
        "get_agent_service_payment_token" => interact.agent_service_payment_token().await,
        "get_agent_service_payment_nonce" => interact.agent_service_payment_nonce().await,
        _ => panic!("unknown command: {}", &cmd),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    contract_address: Option<Bech32Address>,
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
    state: State,
}

impl ContractInteract {
    pub async fn new(config: Config) -> Self {
        let mut interactor = Interactor::new(config.gateway_uri())
            .await
            .use_chain_simulator(config.use_chain_simulator());

        interactor.set_current_dir_from_workspace("identity-registry");
        let wallet_address = interactor.register_wallet(test_wallets::alice()).await;

        // Useful in the chain simulator setting
        // generate blocks until ESDTSystemSCAddress is enabled
        interactor.generate_blocks_until_all_activations().await;

        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/identity-registry.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            wallet_address,
            contract_code,
            state: State::load_state(),
        }
    }

    pub async fn deploy(&mut self) {
        let new_address = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(30_000_000u64)
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .init()
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
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .upgrade()
            .code(&self.contract_code)
            .code_metadata(CodeMetadata::UPGRADEABLE)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn issue_token(&mut self) {
        let egld_amount = BigUint::<StaticApi>::from(0u128);

        let token_display_name = ManagedBuffer::new_from_bytes(&b""[..]);
        let token_ticker = ManagedBuffer::new_from_bytes(&b""[..]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .issue_token(token_display_name, token_ticker)
            .egld(egld_amount)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn register_agent(&mut self) {
        let name = ManagedBuffer::new_from_bytes(&b""[..]);
        let uri = ManagedBuffer::new_from_bytes(&b""[..]);
        let public_key = ManagedBuffer::new_from_bytes(&b""[..]);
        let metadata =
            OptionalValue::Some(ManagedVec::<StaticApi, MetadataEntry<StaticApi>>::new());

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .register_agent(name, uri, public_key, metadata)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn update_agent(&mut self) {
        let token_id = String::new();
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(0u128);

        let new_uri = ManagedBuffer::new_from_bytes(&b""[..]);
        let new_public_key = ManagedBuffer::new_from_bytes(&b""[..]);
        let metadata =
            OptionalValue::Some(ManagedVec::<StaticApi, MetadataEntry<StaticApi>>::new());

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .update_agent(new_uri, new_public_key, metadata)
            .payment((
                EsdtTokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn set_metadata(&mut self) {
        let nonce = 0u64;
        let entries = ManagedVec::<StaticApi, MetadataEntry<StaticApi>>::new();

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .set_metadata(nonce, entries)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn get_metadata(&mut self) {
        let nonce = 0u64;
        let key = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .get_metadata(nonce, key)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn get_agent_service_config(&mut self) {
        let nonce = 0u64;
        let service_id = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .get_agent_service_config(nonce, service_id)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn get_agent(&mut self) {
        let nonce = 0u64;

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .get_agent(nonce)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn get_agent_id(&mut self) {
        let address = ManagedAddress::<StaticApi>::zero();

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .get_agent_id(address)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn agent_token_id(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .agent_token_id()
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn agent_token_nonce(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .agent_token_nonce()
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn agent_owner(&mut self) {
        let nonce = 0u64;

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .agent_owner(nonce)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn agent_service_price(&mut self) {
        let nonce = 0u64;
        let service_id = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .agent_service_price(nonce, service_id)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn agent_service_payment_token(&mut self) {
        let nonce = 0u64;
        let service_id = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .agent_service_payment_token(nonce, service_id)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn agent_service_payment_nonce(&mut self) {
        let nonce = 0u64;
        let service_id = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .agent_service_payment_nonce(nonce, service_id)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }
}
