multiversx_sc::imports!();

use crate::structs::{AgentDetails, JobData};

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

    /// Read agent token ID from identity-registry's NonFungibleTokenMapper.
    #[storage_mapper_from_address("agentTokenId")]
    fn external_agent_token_id(
        &self,
        address: ManagedAddress,
    ) -> SingleValueMapper<TokenIdentifier, ManagedAddress<Self::Api>>;

    /// Read agent details from identity-registry's `agentDetails` storage.
    #[storage_mapper_from_address("agentDetails")]
    fn external_agent_details(
        &self,
        address: ManagedAddress,
        nonce: u64,
    ) -> SingleValueMapper<AgentDetails<Self::Api>, ManagedAddress<Self::Api>>;
}
