multiversx_sc::imports!();

use crate::structs::JobData;

/// Cross-contract storage reads shared across contracts.
#[multiversx_sc::module]
pub trait CrossContractModule {
    /// Read agent owner from identity-registry's `agents` BiDiMapper.
    #[storage_mapper_from_address("agents")]
    fn external_agents(
        &self,
        address: ManagedAddress,
    ) -> BiDiMapper<u64, ManagedAddress, ManagedAddress<Self::Api>>;

    /// Read job data from validation-registry's `jobData` storage.
    #[storage_mapper_from_address("jobData")]
    fn external_job_data(
        &self,
        address: ManagedAddress,
        job_id: &ManagedBuffer,
    ) -> SingleValueMapper<JobData<Self::Api>, ManagedAddress>;

    /// Read agent service config from identity-registry's `agentServiceConfigs` storage.
    #[storage_mapper_from_address("agentServiceConfigs")]
    fn external_agent_service_config(
        &self,
        address: ManagedAddress,
        nonce: u64,
    ) -> MapMapper<u32, Payment<Self::Api>, ManagedAddress<Self::Api>>;
}
