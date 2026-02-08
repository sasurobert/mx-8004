#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod config;
pub mod errors;
pub mod events;
pub mod storage;
pub mod structs;
pub mod views;

pub use structs::*;

use errors::*;

const THREE_DAYS: DurationMillis = DurationMillis::new(3 * 24 * 60 * 60 * 1000);

#[multiversx_sc::contract]
pub trait ValidationRegistry:
    common::cross_contract::CrossContractModule
    + storage::ExternalStorageModule
    + views::ViewsModule
    + events::EventsModule
    + config::ConfigModule
{
    #[init]
    fn init(&self, identity_registry_address: ManagedAddress) {
        self.identity_registry_address()
            .set(&identity_registry_address);
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[payable("*")]
    #[endpoint(initJob)]
    fn init_job(&self, job_id: ManagedBuffer, agent_nonce: u64, service_id: OptionalValue<u32>) {
        let job_mapper = self.job_data(&job_id);
        require!(job_mapper.is_empty(), ERR_JOB_ALREADY_INITIALIZED);

        let caller = self.blockchain().get_caller();
        job_mapper.set(JobData {
            status: JobStatus::New,
            proof: ManagedBuffer::new(),
            employer: caller,
            creation_timestamp: self.blockchain().get_block_timestamp_millis(),
            agent_nonce,
        });

        // If service_id provided, validate payment and forward to agent owner
        if let OptionalValue::Some(sid) = service_id {
            let identity_addr = self.identity_registry_address().get();
            let agent_owner = self.external_agents(identity_addr.clone()).get_value(&agent_nonce);

            let service_config_map = self.external_agent_service_config(identity_addr, agent_nonce);

            if let Some(service_payment) = service_config_map.get(&sid) {
                let pay = self.call_value().single();

                require!(
                    pay.token_identifier == service_payment.token_identifier
                        && pay.token_nonce == service_payment.token_nonce,
                    ERR_INVALID_PAYMENT
                );

                require!(
                    pay.amount >= service_payment.amount,
                    ERR_INSUFFICIENT_PAYMENT
                );

                self.tx().to(&agent_owner).payment(pay).transfer();
            }
        }
    }

    #[endpoint(submitProof)]
    fn submit_proof(&self, job_id: ManagedBuffer, proof: ManagedBuffer) {
        let job_mapper = self.job_data(&job_id);
        require!(!job_mapper.is_empty(), ERR_JOB_NOT_FOUND);

        job_mapper.update(|job| {
            job.proof = proof;
            job.status = JobStatus::Pending;
        });
    }

    #[only_owner]
    #[endpoint(verifyJob)]
    fn verify_job(&self, job_id: ManagedBuffer) {
        let job_mapper = self.job_data(&job_id);
        require!(!job_mapper.is_empty(), ERR_JOB_NOT_FOUND);

        job_mapper.update(|job| {
            job.status = JobStatus::Verified;
            self.job_verified_event(job_id, job.agent_nonce, JobStatus::Verified);
        });
    }

    #[endpoint(cleanOldJobs)]
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
}
