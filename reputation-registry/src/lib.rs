#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod validation_proxy {
    multiversx_sc::imports!();
    #[multiversx_sc::proxy]
    pub trait ValidationRegistry {
        #[view(is_job_verified)]
        fn is_job_verified(&self, job_id: ManagedBuffer) -> bool;

        #[view(getJobEmployer)]
        fn get_job_employer(&self, job_id: ManagedBuffer) -> ManagedAddress;
    }
}

#[multiversx_sc::contract]
pub trait ReputationRegistry {
    #[init]
    fn init(&self) {}

    #[endpoint(submit_feedback)]
    fn submit_feedback(&self, job_id: ManagedBuffer, agent_nonce: u64, rating: BigUint) {
        let caller = self.blockchain().get_caller();
        let validation_addr = self.validation_contract_address().get();

        // 1. Authenticity: Verify job is complete
        let is_verified: bool = self
            .validation_proxy(validation_addr.clone())
            .is_job_verified(job_id.clone())
            .execute_on_dest_context();

        require!(is_verified, "Job not verified");

        // 2. Frontrunning Protection: Verify caller is the employer
        let employer: ManagedAddress = self
            .validation_proxy(validation_addr)
            .get_job_employer(job_id.clone())
            .execute_on_dest_context();

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
        let new_score = (current_score * (total_jobs as u32 - 1) + rating) / total_jobs as u32;

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
        // Only agent should respond
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

    #[proxy]
    fn validation_proxy(&self, sc_address: ManagedAddress) -> validation_proxy::Proxy<Self::Api>;
}
