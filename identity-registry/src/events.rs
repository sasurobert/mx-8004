multiversx_sc::imports!();

use crate::structs::AgentRegisteredEventData;

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("agentRegistered")]
    fn agent_registered_event(
        &self,
        #[indexed] owner: &ManagedAddress,
        #[indexed] nonce: u64,
        data: AgentRegisteredEventData<Self::Api>,
    );

    #[event("agentUpdated")]
    fn agent_updated_event(&self, #[indexed] nonce: u64);

    #[event("metadataUpdated")]
    fn metadata_updated_event(&self, #[indexed] nonce: u64);
}
