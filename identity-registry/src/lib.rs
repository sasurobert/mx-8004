#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(
    TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq, Debug,
)]
pub struct MetadataEntry<M: ManagedTypeApi> {
    pub key: ManagedBuffer<M>,
    pub value: ManagedBuffer<M>,
}

#[type_abi]
#[derive(
    TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq, Debug,
)]
pub struct AgentDetails<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub uri: ManagedBuffer<M>,
    pub public_key: ManagedBuffer<M>,
    pub owner: ManagedAddress<M>,
    pub metadata: ManagedVec<M, MetadataEntry<M>>,
}

#[type_abi]
#[derive(
    TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq, Debug,
)]
pub struct AgentServiceConfig<M: ManagedTypeApi> {
    pub token: EgldOrEsdtTokenIdentifier<M>,
    pub pnonce: u64,
    pub price: BigUint<M>,
}

#[type_abi]
#[derive(
    TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq, Debug,
)]
pub struct AgentRegisteredEventData<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub uri: ManagedBuffer<M>,
}

#[multiversx_sc::contract]
pub trait IdentityRegistry:
    multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issue_token)]
    fn issue_token(&self, token_display_name: ManagedBuffer, token_ticker: ManagedBuffer) {
        require!(self.agent_token_id().is_empty(), "Token already issued");
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

    /// Register a new agent with name, URI, public key, and optional metadata entries.
    ///
    /// # Arguments
    /// * `name` - Display name of the agent
    /// * `uri` - URI pointing to the Agent Registration File (ARF) JSON
    /// * `public_key` - Public key for signature verification
    /// * `metadata` - Optional list of key-value metadata entries (EIP-8004 compatible)
    #[allow_multiple_var_args]
    #[endpoint(register_agent)]
    fn register_agent(
        &self,
        name: ManagedBuffer,
        uri: ManagedBuffer,
        public_key: ManagedBuffer,
        metadata: OptionalValue<ManagedVec<MetadataEntry<Self::Api>>>,
    ) {
        require!(!self.agent_token_id().is_empty(), "Token not issued");

        let caller = self.blockchain().get_caller();
        require!(
            self.agent_id_by_address(&caller).is_empty(),
            "Agent already registered for this address"
        );

        let nonce = self.agent_token_nonce().update(|n| {
            *n += 1;
            *n
        });

        let mut metadata_vec = match metadata {
            OptionalValue::Some(m) => m,
            OptionalValue::None => ManagedVec::new(),
        };

        // If metadata is empty, add a default entry for price:0
        if metadata_vec.is_empty() {
            metadata_vec.push(MetadataEntry {
                key: ManagedBuffer::from("price:0"),
                value: ManagedBuffer::from(BigUint::from(0u64).to_bytes_be()),
            });
        }

        let details = AgentDetails {
            name: name.clone(),
            uri: uri.clone(),
            public_key: public_key.clone(),
            owner: caller.clone(),
            metadata: metadata_vec.clone(),
        };

        self.sync_pricing_metadata(nonce, &metadata_vec);

        // Mint Soulbound NFT
        self.send().esdt_nft_create(
            &self.agent_token_id().get_token_id(),
            &BigUint::from(1u64),
            &name,
            &BigUint::from(0u64),
            &ManagedBuffer::new(),
            &details,
            &self.create_uris_vec(uri.clone()),
        );

        self.agent_id_by_address(&caller).set(nonce);
        self.agent_owner(nonce).set(&caller);
        self.agent_registered_event(&caller, nonce, AgentRegisteredEventData { name, uri });

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

    /// Update an agent's URI, public_key, and optionally metadata.
    /// This follows the Transfer-Execute pattern: the owner sends the agent NFT to this endpoint.
    /// The nonce is automatically extracted from the payment.
    #[payable("*")]
    #[allow_multiple_var_args]
    #[endpoint(update_agent)]
    fn update_agent(
        &self,
        new_uri: ManagedBuffer,
        new_public_key: ManagedBuffer,
        metadata: OptionalValue<ManagedVec<MetadataEntry<Self::Api>>>,
    ) {
        require!(!self.agent_token_id().is_empty(), "Token not issued");

        let payment = self.call_value().single_esdt();
        let token_id = self.agent_token_id().get_token_id();
        require!(payment.token_identifier == token_id, "Invalid NFT sent");

        let nonce = payment.token_nonce;
        let caller = self.blockchain().get_caller();

        let mut details: AgentDetails<Self::Api> =
            self.blockchain().get_token_attributes(&token_id, nonce);

        // Optional updates: if not empty, update
        if !new_uri.is_empty() {
            details.uri = new_uri;
        }
        if !new_public_key.is_empty() {
            details.public_key = new_public_key;
        }
        if let OptionalValue::Some(m) = metadata {
            details.metadata = m;
            self.sync_pricing_metadata(nonce, &details.metadata);
        }

        self.send()
            .nft_update_attributes(&token_id, nonce, &details);

        // Send NFT back to caller
        self.tx()
            .to(&caller)
            .single_esdt(&token_id, nonce, &BigUint::from(1u64))
            .transfer();

        self.agent_updated_event(nonce, &details.uri);
    }

    /// Set or update specific metadata entries for an agent.
    /// This merges with existing metadata (upsert behavior).
    #[endpoint(set_metadata)]
    fn set_metadata(&self, nonce: u64, entries: ManagedVec<MetadataEntry<Self::Api>>) {
        require!(!self.agent_token_id().is_empty(), "Token not issued");

        let caller = self.blockchain().get_caller();
        let token_id = self.agent_token_id().get_token_id();

        let mut details: AgentDetails<Self::Api> =
            self.blockchain().get_token_attributes(&token_id, nonce);
        require!(caller == details.owner, "Only owner can set metadata");

        // Upsert: update existing keys, add new ones
        for entry in entries.iter() {
            let mut found = false;
            let mut new_metadata = ManagedVec::new();
            for existing in details.metadata.iter() {
                if existing.key == entry.key {
                    new_metadata.push(entry.clone());
                    found = true;
                } else {
                    new_metadata.push(existing.clone());
                }
            }
            if !found {
                new_metadata.push(entry.clone());
            }
            details.metadata = new_metadata;
        }

        self.sync_pricing_metadata(nonce, &details.metadata);
        self.send()
            .nft_update_attributes(&token_id, nonce, &details);

        self.metadata_updated_event(nonce);
    }

    fn sync_pricing_metadata(&self, nonce: u64, metadata: &ManagedVec<MetadataEntry<Self::Api>>) {
        let price_prefix = ManagedBuffer::from(b"price:");
        let token_prefix = ManagedBuffer::from(b"token:");
        let pnonce_prefix = ManagedBuffer::from(b"pnonce:");

        for entry in metadata.iter() {
            if entry.key.len() > price_prefix.len() {
                let key_prefix = entry.key.copy_slice(0, price_prefix.len()).unwrap();
                if key_prefix == price_prefix {
                    let service_id = entry
                        .key
                        .copy_slice(price_prefix.len(), entry.key.len() - price_prefix.len())
                        .unwrap();
                    let price = BigUint::top_decode(entry.value.clone())
                        .unwrap_or_else(|_| BigUint::zero());
                    self.agent_service_price(nonce, &service_id).set(&price);
                }
            }

            if entry.key.len() > token_prefix.len() {
                let key_prefix = entry.key.copy_slice(0, token_prefix.len()).unwrap();
                if key_prefix == token_prefix {
                    let service_id = entry
                        .key
                        .copy_slice(token_prefix.len(), entry.key.len() - token_prefix.len())
                        .unwrap();
                    let token_id = EgldOrEsdtTokenIdentifier::top_decode(entry.value.clone())
                        .unwrap_or_else(|_| EgldOrEsdtTokenIdentifier::egld());
                    self.agent_service_payment_token(nonce, &service_id)
                        .set(&token_id);
                }
            }

            if entry.key.len() > pnonce_prefix.len() {
                let key_prefix = entry.key.copy_slice(0, pnonce_prefix.len()).unwrap();
                if key_prefix == pnonce_prefix {
                    let service_id = entry
                        .key
                        .copy_slice(pnonce_prefix.len(), entry.key.len() - pnonce_prefix.len())
                        .unwrap();
                    let p_nonce = u64::top_decode(entry.value.clone()).unwrap_or(0);
                    self.agent_service_payment_nonce(nonce, &service_id)
                        .set(p_nonce);
                }
            }
        }
    }

    /// Get a specific metadata value by key for an agent.
    #[view(get_metadata)]
    fn get_metadata(&self, nonce: u64, key: ManagedBuffer) -> OptionalValue<ManagedBuffer> {
        let token_id = self.agent_token_id().get_token_id();
        let details: AgentDetails<Self::Api> =
            self.blockchain().get_token_attributes(&token_id, nonce);

        for entry in details.metadata.iter() {
            if entry.key == key {
                return OptionalValue::Some(entry.value.clone());
            }
        }
        OptionalValue::None
    }

    /// Get the complete payment configuration for a specific agent service in one call.
    #[view(get_agent_service_config)]
    fn get_agent_service_config(
        &self,
        nonce: u64,
        service_id: ManagedBuffer,
    ) -> AgentServiceConfig<Self::Api> {
        AgentServiceConfig {
            token: self.agent_service_payment_token(nonce, &service_id).get(),
            pnonce: self.agent_service_payment_nonce(nonce, &service_id).get(),
            price: self.agent_service_price(nonce, &service_id).get(),
        }
    }

    fn create_uris_vec(&self, uri: ManagedBuffer) -> ManagedVec<ManagedBuffer> {
        let mut uris = ManagedVec::new();
        uris.push(uri);
        uris
    }

    #[view(get_agent)]
    fn get_agent(&self, nonce: u64) -> AgentDetails<Self::Api> {
        let token_id = self.agent_token_id().get_token_id();
        self.blockchain().get_token_attributes(&token_id, nonce)
    }

    #[view(get_agent_id)]
    fn get_agent_id(&self, address: ManagedAddress) -> u64 {
        self.agent_id_by_address(&address).get()
    }

    // Events

    #[event("agentRegistered")]
    fn agent_registered_event(
        &self,
        #[indexed] owner: &ManagedAddress,
        #[indexed] nonce: u64,
        data: AgentRegisteredEventData<Self::Api>,
    );

    #[event("agentUpdated")]
    fn agent_updated_event(&self, #[indexed] nonce: u64, uri: &ManagedBuffer);

    #[event("metadataUpdated")]
    fn metadata_updated_event(&self, #[indexed] nonce: u64);

    // Storage Mappers

    #[view(get_agent_token_id)]
    #[storage_mapper("agentTokenId")]
    fn agent_token_id(&self) -> NonFungibleTokenMapper;

    #[view(agent_token_nonce)]
    #[storage_mapper("agentTokenNonce")]
    fn agent_token_nonce(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("agentIdByAddress")]
    fn agent_id_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<u64>;

    #[view(get_agent_owner)]
    #[storage_mapper("agentOwner")]
    fn agent_owner(&self, nonce: u64) -> SingleValueMapper<ManagedAddress>;

    #[view(get_agent_service_price)]
    #[storage_mapper("agentServicePrice")]
    fn agent_service_price(
        &self,
        nonce: u64,
        service_id: &ManagedBuffer,
    ) -> SingleValueMapper<BigUint>;

    #[view(get_agent_service_payment_token)]
    #[storage_mapper("agentServicePaymentToken")]
    fn agent_service_payment_token(
        &self,
        nonce: u64,
        service_id: &ManagedBuffer,
    ) -> SingleValueMapper<EgldOrEsdtTokenIdentifier>;

    #[view(get_agent_service_payment_nonce)]
    #[storage_mapper("agentServicePaymentNonce")]
    fn agent_service_payment_nonce(
        &self,
        nonce: u64,
        service_id: &ManagedBuffer,
    ) -> SingleValueMapper<u64>;
}
