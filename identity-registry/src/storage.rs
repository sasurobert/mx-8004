use crate::AgentDetails;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait StorageModule {
    #[view(getAgentTokenId)]
    #[storage_mapper("agentTokenId")]
    fn agent_token_id(&self) -> NonFungibleTokenMapper;

    #[view(getLastAgentNonce)]
    #[storage_mapper("lastAgentNonce")]
    fn agent_last_nonce(&self) -> SingleValueMapper<u64>;

    #[view(getAgentId)]
    #[storage_mapper("agents")]
    fn agents(&self) -> BiDiMapper<u64, ManagedAddress<Self::Api>>;

    #[view(getAgentDetails)]
    #[storage_mapper("agentDetails")]
    fn agent_details(&self, nonce: u64) -> SingleValueMapper<AgentDetails<Self::Api>>;

    #[view(getAgentMetadata)]
    #[storage_mapper("agentMetadatas")]
    fn agent_metadata(&self, nonce: u64) -> MapMapper<ManagedBuffer, ManagedBuffer>;

    #[view(getAgentService)]
    #[storage_mapper("agentServiceConfigs")]
    fn agent_service_config(&self, nonce: u64) -> MapMapper<u32, Payment<Self::Api>>;
}
