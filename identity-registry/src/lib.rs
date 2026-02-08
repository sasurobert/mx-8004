#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod identity_registry_proxy;

// Removed Debug derive to prevent implicit allocations
#[type_abi]
#[derive(TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq)]
pub struct MetadataEntry<M: ManagedTypeApi> {
    pub key: ManagedBuffer<M>,
    pub value: ManagedBuffer<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq)]
pub struct AgentDetails<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub uri: ManagedBuffer<M>,
    pub public_key: ManagedBuffer<M>,
    pub owner: ManagedAddress<M>,
    pub metadata: ManagedVec<M, MetadataEntry<M>>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq)]
pub struct AgentServiceConfig<M: ManagedTypeApi> {
    pub token: EgldOrEsdtTokenIdentifier<M>,
    pub pnonce: u64,
    pub price: BigUint<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, Clone, PartialEq)]
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

        if metadata_vec.is_empty() {
            metadata_vec.push(MetadataEntry {
                key: ManagedBuffer::from("price:0"),
                value: ManagedBuffer::new(), // TopEncoded 0 is empty buffer
            });
        }

        let details = AgentDetails {
            name: name.clone(),
            uri: uri.clone(),
            public_key: public_key.clone(),
            owner: caller.clone(),
            metadata: metadata_vec.clone(),
        };

        // self.sync_pricing_metadata(nonce, &metadata_vec);
        self.sync_pricing_metadata(nonce, &metadata_vec);

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

        self.tx()
            .to(&caller)
            .single_esdt(
                &self.agent_token_id().get_token_id(),
                nonce,
                &BigUint::from(1u64),
            )
            .transfer();
    }

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

        self.tx()
            .to(&caller)
            .single_esdt(&token_id, nonce, &BigUint::from(1u64))
            .transfer();

        self.agent_updated_event(nonce, &details.uri);
    }

    #[endpoint(set_metadata)]
    fn set_metadata(&self, nonce: u64, entries: ManagedVec<MetadataEntry<Self::Api>>) {
        require!(!self.agent_token_id().is_empty(), "Token not issued");

        let caller = self.blockchain().get_caller();
        let token_id = self.agent_token_id().get_token_id();

        let mut details: AgentDetails<Self::Api> =
            self.blockchain().get_token_attributes(&token_id, nonce);
        require!(caller == details.owner, "Only owner can set metadata");

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
        // Use byte comparisons instead of ManagedBuffer creation via copy_slice where possible
        for entry in metadata.iter() {
            // Check "price:" prefix (len 6)
            if entry.key.len() > 6 {
                let mut prefix_buf = [0u8; 6];
                let _ = entry.key.load_slice(0, &mut prefix_buf);
                if &prefix_buf == b"price:" {
                    if let Some(service_id) = entry.key.copy_slice(6, entry.key.len() - 6) {
                        let price = BigUint::top_decode(entry.value.clone())
                            .unwrap_or_else(|_| BigUint::zero());
                        self.agent_service_price(nonce, &service_id).set(&price);
                    }
                }
            }

            // Check "token:" prefix (len 6)
            if entry.key.len() > 6 {
                let mut prefix_buf = [0u8; 6];
                let _ = entry.key.load_slice(0, &mut prefix_buf);
                if &prefix_buf == b"token:" {
                    if let Some(service_id) = entry.key.copy_slice(6, entry.key.len() - 6) {
                        let token_id = EgldOrEsdtTokenIdentifier::top_decode(entry.value.clone())
                            .unwrap_or_else(|_| EgldOrEsdtTokenIdentifier::egld());
                        self.agent_service_payment_token(nonce, &service_id)
                            .set(&token_id);
                    }
                }
            }

            // Check "pnonce:" prefix (len 7)
            if entry.key.len() > 7 {
                let mut prefix_buf = [0u8; 7];
                let _ = entry.key.load_slice(0, &mut prefix_buf);
                if &prefix_buf == b"pnonce:" {
                    if let Some(service_id) = entry.key.copy_slice(7, entry.key.len() - 7) {
                        let p_nonce = u64::top_decode(entry.value.clone()).unwrap_or(0);
                        self.agent_service_payment_nonce(nonce, &service_id)
                            .set(p_nonce);
                    }
                }
            }
        }
    }

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
