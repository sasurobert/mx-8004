multiversx_sc::imports!();

use crate::structs::JobData;

#[multiversx_sc::module]
pub trait ExternalStorageModule: common::cross_contract::CrossContractModule {
    // ── Local storage ──

    #[storage_mapper("jobData")]
    fn job_data(&self, job_id: &ManagedBuffer) -> SingleValueMapper<JobData<Self::Api>>;

    #[storage_mapper("identityRegistryAddress")]
    fn identity_registry_address(&self) -> SingleValueMapper<ManagedAddress>;
}
