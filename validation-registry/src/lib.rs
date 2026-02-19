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
    #[endpoint(init_job)]
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
            let agent_owner = self
                .external_agents(identity_addr.clone())
                .get_value(&agent_nonce);

            let service_config_map = self.external_agent_service_config(identity_addr, agent_nonce);

            if let Some(service_payment) = service_config_map.get(&sid) {
                if let Some(pay) = self.call_value().single_optional() {
                    require!(
                        pay.token_identifier == service_payment.token_identifier
                            && pay.token_nonce == service_payment.token_nonce,
                        ERR_INVALID_PAYMENT
                    );

                    require!(
                        pay.amount >= service_payment.amount,
                        ERR_INSUFFICIENT_PAYMENT
                    );

                    if pay.amount > 0u64 {
                        self.tx().to(&agent_owner).payment(pay.clone()).transfer();
                    }
                } else {
                    // No payment sent — only valid if service is free
                    require!(service_payment.amount == 0u64, ERR_INSUFFICIENT_PAYMENT);
                }
            }
        }
    }

    #[endpoint(submit_proof)]
    fn submit_proof(&self, job_id: ManagedBuffer, proof: ManagedBuffer) {
        let job_mapper = self.job_data(&job_id);
        require!(!job_mapper.is_empty(), ERR_JOB_NOT_FOUND);

        let job_data = job_mapper.get();

        // Security Check 1: Agent Owner or Registered Agent
        let caller = self.blockchain().get_caller();
        let identity_addr = self.identity_registry_address().get();

        // Optimization: Check Agent first (most frequent caller)
        let agent_details = self
            .external_agent_details(identity_addr.clone(), job_data.agent_nonce)
            .get();

        let mut is_authorized = false;
        if caller.as_managed_buffer() == &agent_details.public_key {
            is_authorized = true;
        } else {
            // Fallback: Check Agent Owner
            let agent_owner = self
                .external_agents(identity_addr)
                .get_value(&job_data.agent_nonce);

            if caller == agent_owner {
                is_authorized = true;
            }
        }
        require!(is_authorized, ERR_NOT_AGENT_OWNER);

        // Security Check 2: Ensure job is in correct state
        require!(
            job_data.status == JobStatus::New || job_data.status == JobStatus::Pending,
            ERR_JOB_STATUS_INVALID
        );

        job_mapper.update(|job| {
            job.proof = proof;
            job.status = JobStatus::Pending;
        });
    }

    /// NFT-holder variant: proves ownership by sending the agent NFT.
    /// The contract verifies token ID + nonce, executes proof, and returns the NFT.
    #[payable("*")]
    #[endpoint(submit_proof_with_nft)]
    fn submit_proof_with_nft(&self, job_id: ManagedBuffer, proof: ManagedBuffer) {
        let job_mapper = self.job_data(&job_id);
        require!(!job_mapper.is_empty(), ERR_JOB_NOT_FOUND);

        let payment = self.call_value().single_esdt();
        let job_data = job_mapper.get();

        // Security Check: Ensure job is in correct state
        require!(
            job_data.status == JobStatus::New || job_data.status == JobStatus::Pending,
            ERR_JOB_STATUS_INVALID
        );

        // Read agent token ID from identity-registry
        let identity_addr = self.identity_registry_address().get();
        let expected_token_id = self.external_agent_token_id(identity_addr).get();
        require!(
            payment.token_identifier == expected_token_id,
            ERR_INVALID_AGENT_NFT
        );
        require!(
            payment.token_nonce == job_data.agent_nonce,
            ERR_INVALID_AGENT_NFT
        );

        job_mapper.update(|job| {
            job.proof = proof;
            job.status = JobStatus::Pending;
        });

        // Return NFT to caller
        let caller = self.blockchain().get_caller();
        self.tx()
            .to(&caller)
            .single_esdt(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            )
            .transfer();
    }

    /// ERC-8004: Agent requests validation from a specific validator.
    /// MUST be called by the owner of the agent (agentId).
    #[endpoint(validation_request)]
    fn validation_request(
        &self,
        job_id: ManagedBuffer,
        validator_address: ManagedAddress,
        request_uri: ManagedBuffer,
        request_hash: ManagedBuffer,
    ) {
        let job_mapper = self.job_data(&job_id);
        require!(!job_mapper.is_empty(), ERR_JOB_NOT_FOUND);

        let job_data = job_mapper.get();

        // Caller must be agent owner
        let caller = self.blockchain().get_caller();
        let identity_addr = self.identity_registry_address().get();
        let agent_owner = self
            .external_agents(identity_addr)
            .get_value(&job_data.agent_nonce);
        require!(caller == agent_owner, ERR_NOT_AGENT_OWNER);

        // Store validation request
        let request_data = ValidationRequestData {
            validator_address: validator_address.clone(),
            agent_nonce: job_data.agent_nonce,
            job_id: job_id.clone(),
            response: 0,
            response_hash: ManagedBuffer::new(),
            tag: ManagedBuffer::new(),
            last_update: TimestampSeconds::new(0),
        };

        self.validation_request_data(&request_hash)
            .set(&request_data);
        self.agent_validations(job_data.agent_nonce)
            .insert(request_hash.clone());

        // Update job status
        job_mapper.update(|job| {
            job.status = JobStatus::ValidationRequested;
        });

        self.validation_request_event(
            validator_address,
            job_data.agent_nonce,
            request_uri,
            request_hash,
        );
    }

    /// ERC-8004: Validator responds with a result (0-100).
    /// MUST be called by the validatorAddress from the original request.
    /// Can be called multiple times for progressive validation.
    #[endpoint(validation_response)]
    fn validation_response(
        &self,
        request_hash: ManagedBuffer,
        response: u8,
        _response_uri: ManagedBuffer,
        response_hash: ManagedBuffer,
        tag: ManagedBuffer,
    ) {
        let request_mapper = self.validation_request_data(&request_hash);
        require!(!request_mapper.is_empty(), ERR_VALIDATION_REQUEST_NOT_FOUND);

        let caller = self.blockchain().get_caller();

        request_mapper.update(|data| {
            require!(caller == data.validator_address, ERR_NOT_VALIDATOR);

            data.response = response;
            data.response_hash = response_hash;
            data.tag = tag;
            data.last_update = self.blockchain().get_block_timestamp_seconds();
        });

        let updated_data = request_mapper.get();

        // Transition job status to Verified
        let job_mapper = self.job_data(&updated_data.job_id);
        if !job_mapper.is_empty() {
            job_mapper.update(|job| {
                job.status = JobStatus::Verified;
            });
        }

        self.validation_response_event(
            caller,
            updated_data.agent_nonce,
            request_hash,
            updated_data,
        );
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
}
