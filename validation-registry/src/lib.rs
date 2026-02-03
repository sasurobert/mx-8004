#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

const THREE_DAYS: DurationMillis = DurationMillis::new(3 * 24 * 60 * 60 * 1000);

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub enum JobStatus {
    New,
    Pending,
    Verified,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct JobData<M: ManagedTypeApi> {
    pub status: JobStatus,
    pub proof: ManagedBuffer<M>,
    pub employer: ManagedAddress<M>,
    pub creation_timestamp: TimestampMillis,
    pub agent_nonce: u64,
}

#[multiversx_sc::contract]
pub trait ValidationRegistry {
    #[init]
    fn init(&self) {}

    #[endpoint(init_job)]
    fn init_job(&self, job_id: ManagedBuffer, agent_nonce: u64) {
        let job_mapper = self.job_data(&job_id);
        require!(job_mapper.is_empty(), "Job already initialized");

        job_mapper.set(JobData {
            status: JobStatus::New,
            proof: ManagedBuffer::new(),
            employer: self.blockchain().get_caller(),
            creation_timestamp: self.blockchain().get_block_timestamp_millis(),
            agent_nonce,
        });
    }

    #[endpoint(submit_proof)]
    fn submit_proof(&self, job_id: ManagedBuffer, proof: ManagedBuffer) {
        let job_mapper = self.job_data(&job_id);
        require!(!job_mapper.is_empty(), "Job does not exist");

        // In a real scenario, we'd check if caller owns agent_nonce
        // For this standard, we assume the agent submitter is verified via identity

        job_mapper.update(|job| {
            job.proof = proof;
            job.status = JobStatus::Pending;
        });
    }

    #[only_owner]
    #[endpoint(verify_job)]
    fn verify_job(&self, job_id: ManagedBuffer) {
        let job_mapper = self.job_data(&job_id);
        require!(!job_mapper.is_empty(), "Job does not exist");

        job_mapper.update(|job| {
            // Access control for Oracle is assumed for now
            job.status = JobStatus::Verified;

            self.job_verified_event(job_id, job.agent_nonce, JobStatus::Verified);
        });
    }

    #[endpoint(clean_old_jobs)]
    fn clean_old_jobs(&self, job_ids: MultiValueEncoded<ManagedBuffer>) {
        let current_time = self.blockchain().get_block_timestamp_millis();
        for job_id in job_ids {
            let job_mapper = self.job_data(&job_id);
            if job_mapper.is_empty() {
                continue;
            }
            let job_data = job_mapper.get();
            if current_time > job_data.creation_timestamp + THREE_DAYS {
                job_mapper.clear();
            }
        }
    }

    #[view(is_job_verified)]
    fn is_job_verified(&self, job_id: ManagedBuffer) -> bool {
        let job_mapper = self.job_data(&job_id);
        !job_mapper.is_empty() && job_mapper.get().status == JobStatus::Verified
    }

    #[view(getJobData)]
    fn get_job_data(&self, job_id: ManagedBuffer) -> OptionalValue<JobData<Self::Api>> {
        let job_mapper = self.job_data(&job_id);
        if job_mapper.is_empty() {
            OptionalValue::None
        } else {
            OptionalValue::Some(job_mapper.get())
        }
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
    #[storage_mapper("jobData")]
    fn job_data(&self, job_id: &ManagedBuffer) -> SingleValueMapper<JobData<Self::Api>>;
}
