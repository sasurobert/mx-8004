multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub use common::structs::{JobData, JobStatus};

#[multiversx_sc::module]
pub trait StorageModule: common::cross_contract::CrossContractModule {
    // ── Local storage ──

    #[view(getReputationScore)]
    #[storage_mapper("reputationScore")]
    fn reputation_score(&self, agent_nonce: u64) -> SingleValueMapper<BigUint>;

    #[view(getTotalJobs)]
    #[storage_mapper("totalJobs")]
    fn total_jobs(&self, agent_nonce: u64) -> SingleValueMapper<u64>;

    #[view(getValidationContractAddress)]
    #[storage_mapper("validationContractAddress")]
    fn validation_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getIdentityContractAddress)]
    #[storage_mapper("identityContractAddress")]
    fn identity_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(hasGivenFeedback)]
    #[storage_mapper("hasGivenFeedback")]
    fn has_given_feedback(&self, job_id: ManagedBuffer) -> SingleValueMapper<bool>;

    #[view(isFeedbackAuthorized)]
    #[storage_mapper("isFeedbackAuthorized")]
    fn is_feedback_authorized(
        &self,
        job_id: ManagedBuffer,
        client: ManagedAddress,
    ) -> SingleValueMapper<bool>;

    #[view(getAgentResponse)]
    #[storage_mapper("agentResponse")]
    fn agent_response(&self, job_id: ManagedBuffer) -> SingleValueMapper<ManagedBuffer>;
}
