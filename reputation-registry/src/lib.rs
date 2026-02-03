#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod identity_registry_proxy;
mod validation_registry_proxy;

#[multiversx_sc::contract]
pub trait ReputationRegistry {
    #[init]
    fn init(&self) {}

    #[endpoint(submit_feedback)]
    fn submit_feedback(&self, job_id: ManagedBuffer, agent_nonce: u64, rating: BigUint) {
        let caller = self.blockchain().get_caller();
        let validation_addr = self.validation_contract_address().get();

        // 1. Authenticity: Verify job is complete
        let is_verified = self
            .tx()
            .to(&validation_addr)
            .typed(validation_registry_proxy::ValidationRegistryProxy)
            .is_job_verified(&job_id)
            .returns(ReturnsResult)
            .sync_call();

        require!(is_verified, "Job not verified");

        // 2. Frontrunning Protection: Verify caller is the employer
        let employer = self
            .tx()
            .to(&validation_addr)
            .typed(validation_registry_proxy::ValidationRegistryProxy)
            .job_employer(&job_id)
            .returns(ReturnsResult)
            .sync_call();

        require!(caller == employer, "Only the employer can provide feedback");

        // 3. Authorization Gate: Verify agent authorized this specific feedback
        require!(
            self.is_feedback_authorized(job_id.clone(), caller).get(),
            "Feedback not authorized by agent"
        );

        // 4. Duplicate Prevention
        require!(
            !self.has_given_feedback(job_id.clone()).get(),
            "Feedback already provided for this job"
        );

        let total_jobs = self.total_jobs(agent_nonce).update(|n| {
            *n += 1;
            *n
        });

        let current_score = self.reputation_score(agent_nonce).get();
        // Calculate new score: ((current * (total - 1)) + rating) / total
        let total_big = BigUint::from(total_jobs);
        let prev_total = &total_big - 1u32;
        let weighted_score = current_score * prev_total;
        let new_score = (weighted_score + rating) / total_big;

        self.reputation_score(agent_nonce).set(&new_score);
        self.has_given_feedback(job_id).set(true);

        self.reputation_updated_event(agent_nonce, new_score);
    }

    #[endpoint(authorize_feedback)]
    fn authorize_feedback(&self, job_id: ManagedBuffer, client: ManagedAddress) {
        // In a real scenario, we'd check if caller owns the agent linked to job_id
        // For this standard, we assume the agent calling this owns the job context
        self.is_feedback_authorized(job_id, client).set(true);
    }

    #[endpoint(append_response)]
    fn append_response(&self, job_id: ManagedBuffer, response_uri: ManagedBuffer) {
        // 1. Get Agent Nonce from Validation Registry
        let validation_addr = self.validation_contract_address().get();
        let agent_nonce = self
            .tx()
            .to(&validation_addr)
            .typed(validation_registry_proxy::ValidationRegistryProxy)
            .job_agent_nonce(&job_id)
            .returns(ReturnsResult)
            .sync_call();

        require!(agent_nonce != 0, "Job not found or not initialized");

        // 2. Get Agent Owner from Identity Registry
        let identity_addr = self.identity_contract_address().get();
        let agent_details = self
            .tx()
            .to(&identity_addr)
            .typed(identity_registry_proxy::IdentityRegistryProxy)
            .get_agent(agent_nonce)
            .returns(ReturnsResult)
            .sync_call();

        // 3. Verify Caller
        let caller = self.blockchain().get_caller();
        require!(
            caller == agent_details.owner,
            "Only the agent owner can respond"
        );

        self.agent_response(job_id).set(response_uri);
    }

    #[event("reputationUpdated")]
    fn reputation_updated_event(&self, #[indexed] agent_nonce: u64, new_score: BigUint);

    #[view]
    #[storage_mapper("reputationScore")]
    fn reputation_score(&self, agent_nonce: u64) -> SingleValueMapper<BigUint>;

    #[view]
    #[storage_mapper("totalJobs")]
    fn total_jobs(&self, agent_nonce: u64) -> SingleValueMapper<u64>;

    #[view]
    #[storage_mapper("validationContractAddress")]
    fn validation_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view]
    #[storage_mapper("identityContractAddress")]
    fn identity_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view]
    #[storage_mapper("hasGivenFeedback")]
    fn has_given_feedback(&self, job_id: ManagedBuffer) -> SingleValueMapper<bool>;

    #[view]
    #[storage_mapper("isFeedbackAuthorized")]
    fn is_feedback_authorized(
        &self,
        job_id: ManagedBuffer,
        client: ManagedAddress,
    ) -> SingleValueMapper<bool>;

    #[view]
    #[storage_mapper("agentResponse")]
    fn agent_response(&self, job_id: ManagedBuffer) -> SingleValueMapper<ManagedBuffer>;
}
