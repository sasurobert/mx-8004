#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(TopEncode, TopDecode, ManagedVecItem, NestedEncode, NestedDecode, PartialEq, Debug)]
pub enum JobStatus {
    Pending,
    Verified,
}

#[multiversx_sc::contract]
pub trait ValidationRegistry {
    #[init]
    fn init(&self) {}

    #[endpoint(init_job)]
    fn init_job(&self, job_id: ManagedBuffer, agent_nonce: u64) {
        require!(
            self.job_employer(job_id.clone()).is_empty(),
            "Job already initialized"
        );
        self.job_employer(job_id.clone())
            .set(self.blockchain().get_caller());
        self.job_creation_timestamp(job_id.clone())
            .set(self.blockchain().get_block_timestamp_seconds());
        self.job_agent_nonce(job_id).set(agent_nonce);
    }

    #[endpoint(submit_proof)]
    fn submit_proof(&self, job_id: ManagedBuffer, proof: ManagedBuffer) {
        let agent_nonce = self.job_agent_nonce(job_id.clone()).get();
        require!(agent_nonce != 0, "Job not initialized");

        // In a real scenario, we'd check if caller owns agent_nonce
        // For this standard, we assume the agent submitter is verified via identity

        self.job_proof(job_id.clone()).set(&proof);
        self.job_status(job_id).set(JobStatus::Pending);
    }

    #[only_owner]
    #[endpoint(verify_job)]
    fn verify_job(&self, job_id: ManagedBuffer) {
        // Access control for Oracle is assumed for now
        self.job_status(job_id.clone()).set(JobStatus::Verified);

        let agent_nonce = self.job_agent_nonce(job_id.clone()).get();
        self.job_verified_event(job_id, agent_nonce, JobStatus::Verified);
    }

    #[endpoint(clean_old_jobs)]
    fn clean_old_jobs(&self, job_ids: MultiValueEncoded<ManagedBuffer>) {
        let current_time = self.blockchain().get_block_timestamp_seconds();
        let three_days = DurationSeconds::new(3 * 24 * 60 * 60);
        for job_id in job_ids {
            let ts = self.job_creation_timestamp(job_id.clone()).get();
            if ts > TimestampSeconds::new(0) && current_time > ts + three_days {
                self.job_proof(job_id.clone()).clear();
                self.job_status(job_id.clone()).clear();
                self.job_employer(job_id.clone()).clear();
                self.job_creation_timestamp(job_id.clone()).clear();
                self.job_agent_nonce(job_id).clear();
            }
        }
    }

    #[view(is_job_verified)]
    fn is_job_verified(&self, job_id: ManagedBuffer) -> bool {
        self.job_status(job_id).get() == JobStatus::Verified
    }

    // Events
    #[event("jobVerified")]
    fn job_verified_event(
        &self,
        #[indexed] job_id: ManagedBuffer,
        #[indexed] agent_nonce: u64,
        status: JobStatus,
    );

    // Storage Mappers
    #[view(getJobProof)]
    #[storage_mapper("jobProof")]
    fn job_proof(&self, job_id: ManagedBuffer) -> SingleValueMapper<ManagedBuffer>;

    #[view(getJobStatus)]
    #[storage_mapper("jobStatus")]
    fn job_status(&self, job_id: ManagedBuffer) -> SingleValueMapper<JobStatus>;

    #[view(getJobEmployer)]
    #[storage_mapper("jobEmployer")]
    fn job_employer(&self, job_id: ManagedBuffer) -> SingleValueMapper<ManagedAddress>;

    #[view(getJobCreationTimestamp)]
    #[storage_mapper("jobCreationTimestamp")]
    fn job_creation_timestamp(&self, job_id: ManagedBuffer) -> SingleValueMapper<TimestampSeconds>;

    #[view(getJobAgentNonce)]
    #[storage_mapper("jobAgentNonce")]
    fn job_agent_nonce(&self, job_id: ManagedBuffer) -> SingleValueMapper<u64>;
}
