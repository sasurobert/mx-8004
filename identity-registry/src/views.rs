multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::structs::AgentDetails;

#[multiversx_sc::module]
pub trait ViewsModule: crate::storage::StorageModule {
    #[view(get_agent)]
    fn get_agent(&self, nonce: u64) -> AgentDetails<Self::Api> {
        self.agent_details(nonce).get()
    }

    #[view(get_agent_owner)]
    fn get_agent_owner(&self, nonce: u64) -> ManagedAddress {
        self.agents().get_value(&nonce)
    }

    #[view(get_metadata)]
    fn get_metadata(&self, nonce: u64, key: ManagedBuffer) -> OptionalValue<ManagedBuffer> {
        let mapper = self.agent_metadata(nonce);
        if let Some(value) = mapper.get(&key) {
            OptionalValue::Some(value)
        } else {
            OptionalValue::None
        }
    }

    #[view(get_agent_service_config)]
    fn get_agent_service_config(
        &self,
        nonce: u64,
        service_id: u32,
    ) -> OptionalValue<Payment<Self::Api>> {
        let mapper = self.agent_service_config(nonce);
        if let Some(payment) = mapper.get(&service_id) {
            OptionalValue::Some(payment)
        } else {
            OptionalValue::None
        }
    }
}
