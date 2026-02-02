#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode)]
pub struct AgentDetails<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub uri: ManagedBuffer<M>,
    pub public_key: ManagedBuffer<M>,
    pub owner: ManagedAddress<M>,
}

#[type_abi]
#[derive(TopEncode)]
pub struct AgentRegisteredEventData<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub uri: ManagedBuffer<M>,
}

#[multiversx_sc::contract]
pub trait IdentityRegistry {
    #[init]
    fn init(&self) {}

    #[endpoint(register_agent)]
    fn register_agent(&self, name: ManagedBuffer, uri: ManagedBuffer, public_key: ManagedBuffer) {
        let nonce = self.agent_uri_nonce().update(|n| {
            *n += 1;
            *n
        });

        self.agent_uri(nonce).set(&uri);
        self.agent_public_key(nonce).set(&public_key);
        self.agent_name(nonce).set(&name);
        self.agent_owner(nonce).set(self.blockchain().get_caller());

        self.agent_registered_event(
            &self.blockchain().get_caller(),
            nonce,
            AgentRegisteredEventData { name, uri },
        );
    }

    #[endpoint(update_agent)]
    fn update_agent(&self, nonce: u64, new_uri: ManagedBuffer, new_public_key: ManagedBuffer) {
        let caller = self.blockchain().get_caller();
        let owner = self.agent_owner(nonce).get();
        require!(caller == owner, "Only owner can update agent");

        self.agent_uri(nonce).set(&new_uri);
        self.agent_public_key(nonce).set(&new_public_key);

        self.agent_updated_event(nonce, &new_uri);
    }

    #[view(getAgent)]
    fn get_agent(&self, nonce: u64) -> AgentDetails<Self::Api> {
        AgentDetails {
            name: self.agent_name(nonce).get(),
            uri: self.agent_uri(nonce).get(),
            public_key: self.agent_public_key(nonce).get(),
            owner: self.agent_owner(nonce).get(),
        }
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

    // Storage Mappers

    #[view]
    #[storage_mapper("agentNftTokenId")]
    fn agent_nft_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(agent_uri_nonce)]
    #[storage_mapper("agentUriNonce")]
    fn agent_uri_nonce(&self) -> SingleValueMapper<u64>;

    #[view]
    #[storage_mapper("agentUri")]
    fn agent_uri(&self, nonce: u64) -> SingleValueMapper<ManagedBuffer>;

    #[view]
    #[storage_mapper("agentPublicKey")]
    fn agent_public_key(&self, nonce: u64) -> SingleValueMapper<ManagedBuffer>;

    #[view]
    #[storage_mapper("agentName")]
    fn agent_name(&self, nonce: u64) -> SingleValueMapper<ManagedBuffer>;

    #[view]
    #[storage_mapper("agentOwner")]
    fn agent_owner(&self, nonce: u64) -> SingleValueMapper<ManagedAddress>;
}
