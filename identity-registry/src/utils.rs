use crate::errors::*;
use crate::structs::*;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UtilsModule: crate::storage::StorageModule {
    fn require_agent_owner(&self, nonce: u64) -> ManagedAddress {
        let caller = self.blockchain().get_caller();
        let owner = self.agents().get_value(&nonce);
        require!(caller == owner, ERR_NOT_OWNER);
        caller
    }

    fn sync_metadata(
        &self,
        nonce: u64,
        entries: MultiValueEncodedCounted<MetadataEntry<Self::Api>>,
    ) {
        for entry in entries {
            self.agent_metadata(nonce).insert(entry.key, entry.value);
        }
    }

    fn sync_service_configs(
        &self,
        nonce: u64,
        configs: MultiValueEncodedCounted<ServiceConfigInput<Self::Api>>,
    ) {
        for config in configs {
            if config.price > 0 {
                let amount = NonZeroBigUint::new_or_panic(config.price);
                let payment = Payment::new(config.token, config.nonce, amount);
                self.agent_service_config(nonce)
                    .insert(config.service_id, payment);
            } else {
                self.agent_service_config(nonce).remove(&config.service_id);
            }
        }
    }

    fn create_uris_vec(&self, uri: ManagedBuffer) -> ManagedVec<ManagedBuffer> {
        let mut uris = ManagedVec::new();
        uris.push(uri);
        uris
    }
}
