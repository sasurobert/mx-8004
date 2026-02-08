#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod errors;
pub mod events;
pub mod storage;
pub mod structs;
pub mod utils;
pub mod views;

pub use structs::*;

use errors::*;

#[multiversx_sc::contract]
pub trait IdentityRegistry:
    multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + storage::StorageModule
    + views::ViewsModule
    + events::EventsModule
    + utils::UtilsModule
{
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueToken)]
    fn issue_token(&self, token_display_name: ManagedBuffer, token_ticker: ManagedBuffer) {
        require!(self.agent_token_id().is_empty(), ERR_TOKEN_ALREADY_ISSUED);
        let issue_cost = self.call_value().egld().clone_value();

        self.agent_token_id().issue_and_set_all_roles(
            EsdtTokenType::NonFungible,
            issue_cost,
            token_display_name,
            token_ticker,
            0,
            None,
        );
    }

    /// Register a new agent with name, URI, public key, optional metadata, and optional service configs.
    #[allow_multiple_var_args]
    #[endpoint(registerAgent)]
    fn register_agent(
        &self,
        name: ManagedBuffer,
        uri: ManagedBuffer,
        public_key: ManagedBuffer,
        metadata: MultiValueEncodedCounted<MetadataEntry<Self::Api>>,
        services: MultiValueEncodedCounted<ServiceConfigInput<Self::Api>>,
    ) {
        require!(!self.agent_token_id().is_empty(), ERR_TOKEN_NOT_ISSUED);

        let caller = self.blockchain().get_caller();
        require!(
            !self.agents().contains_value(&caller),
            ERR_AGENT_ALREADY_REGISTERED
        );

        let details = AgentDetails {
            name: name.clone(),
            public_key,
        };

        // Mint soulbound NFT — proof of agent identity
        let nonce = self.send().esdt_nft_create(
            &self.agent_token_id().get_token_id(),
            &BigUint::from(1u64),
            &name,
            &BigUint::from(0u64),
            &ManagedBuffer::new(),
            &details,
            &ManagedVec::from_single_item(uri.clone()),
        );

        // Store all data in storage mappers
        self.agents().insert(nonce, caller.clone());
        self.agent_details(nonce).set(&details);
        self.agent_last_nonce().set(nonce);

        // Store metadata if provided
        self.sync_metadata(nonce, metadata);

        self.sync_service_configs(nonce, services);

        self.agent_registered_event(
            &caller,
            nonce,
            AgentRegisteredEventData {
                name: details.name,
                uri: uri.clone(),
            },
        );

        // Send NFT to caller
        self.tx()
            .to(&caller)
            .single_esdt(
                &self.agent_token_id().get_token_id(),
                nonce,
                &BigUint::from(1u64),
            )
            .transfer();
    }

    /// Update an agent's URI and/or public_key. Requires sending the agent NFT.
    #[payable("*")]
    #[allow_multiple_var_args]
    #[endpoint(updateAgent)]
    fn update_agent(
        &self,
        new_name: ManagedBuffer,
        new_uri: ManagedBuffer,
        new_public_key: ManagedBuffer,
        signature: ManagedBuffer,
        metadata: OptionalValue<MultiValueEncodedCounted<MetadataEntry<Self::Api>>>,
        services: OptionalValue<MultiValueEncodedCounted<ServiceConfigInput<Self::Api>>>,
    ) {
        require!(!self.agent_token_id().is_empty(), ERR_TOKEN_NOT_ISSUED);

        let payment = self.call_value().single_esdt();
        let token_id = self.agent_token_id().get_token_id();
        require!(payment.token_identifier == token_id, ERR_INVALID_NFT);

        let nonce = payment.token_nonce;
        let caller = self.blockchain().get_caller();
        let owner = self.agents().get_value(&nonce);
        require!(caller == owner, ERR_NOT_OWNER);

        let hashed = self.crypto().sha256(new_public_key.clone());
        self.crypto()
            .verify_ed25519(&new_public_key, hashed.as_managed_buffer(), &signature);

        self.send().esdt_metadata_recreate(
            token_id.clone(),
            nonce,
            new_name,
            0,
            new_public_key,
            &ManagedBuffer::new(),
            ManagedVec::from_single_item(new_uri),
        );

        if let OptionalValue::Some(m) = metadata {
            self.sync_metadata(nonce, m);
        }
        if let OptionalValue::Some(configs) = services {
            self.sync_service_configs(nonce, configs);
        }

        // Send NFT back to caller
        self.tx()
            .to(&caller)
            .single_esdt(&token_id, nonce, &BigUint::from(1u64))
            .transfer();

        self.agent_updated_event(nonce);
    }

    /// Set or update metadata entries for an agent. O(1) per entry via MapMapper.
    #[endpoint(setMetadata)]
    fn set_metadata(
        &self,
        nonce: u64,
        entries: MultiValueEncodedCounted<MetadataEntry<Self::Api>>,
    ) {
        require!(!self.agent_token_id().is_empty(), ERR_TOKEN_NOT_ISSUED);
        self.require_agent_owner(nonce);
        self.sync_metadata(nonce, entries);
        self.metadata_updated_event(nonce);
    }

    /// Set or update service configurations for an agent.
    #[endpoint(setServiceConfigs)]
    fn set_service_configs_endpoint(
        &self,
        nonce: u64,
        configs: MultiValueEncodedCounted<ServiceConfigInput<Self::Api>>,
    ) {
        require!(!self.agent_token_id().is_empty(), ERR_TOKEN_NOT_ISSUED);
        self.require_agent_owner(nonce);
        self.sync_service_configs(nonce, configs);
    }
}
