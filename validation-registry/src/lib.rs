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
    fn init(&self, identity_registry_address: ManagedAddress) {
        self.identity_registry_address()
            .set(&identity_registry_address);
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[only_owner]
    #[endpoint(set_identity_registry_address)]
    fn set_identity_registry_address(&self, address: ManagedAddress) {
        self.identity_registry_address().set(&address);
    }

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

    /// Optimized job initialization with payment.
    /// Uses direct storage reads from IdentityRegistry for maximum performance.
    #[payable("*")]
    #[endpoint(init_job_with_payment)]
    fn init_job_with_payment(
        &self,
        job_id: ManagedBuffer,
        agent_nonce: u64,
        service_id: ManagedBuffer,
    ) {
        let job_mapper = self.job_data(&job_id);
        require!(job_mapper.is_empty(), "Job already initialized");

        let identity_addr = self.identity_registry_address().get();

        // Resolve agent owner and price via direct storage access
        let agent_owner: ManagedAddress =
            self.read_owner_from_identity(&identity_addr, agent_nonce);
        let required_price: BigUint =
            self.read_price_from_identity(&identity_addr, agent_nonce, &service_id);

        let payment = self.call_value().all();

        let mut total_paid = BigUint::zero();
        for pay in payment.iter() {
            total_paid += pay.amount.clone().into_big_uint();
        }

        require!(total_paid >= required_price, "Insufficient payment");

        // Forward payment to agent owner using Unified Transaction API
        self.tx().to(&agent_owner).payment(payment).transfer();

        // Register the job
        job_mapper.set(JobData {
            status: JobStatus::New,
            proof: ManagedBuffer::new(),
            employer: self.blockchain().get_caller(),
            creation_timestamp: self.blockchain().get_block_timestamp_millis(),
            agent_nonce,
        });
    }

    fn read_owner_from_identity(&self, addr: &ManagedAddress, nonce: u64) -> ManagedAddress {
        let mut key = ManagedBuffer::from(b"agentOwner");
        let _ = nonce.dep_encode(&mut key);

        self.storage_raw().read_from_address(addr, key)
    }

    fn read_price_from_identity(
        &self,
        addr: &ManagedAddress,
        nonce: u64,
        service_id: &ManagedBuffer,
    ) -> BigUint {
        let mut key = ManagedBuffer::from(b"agentServicePrice");
        let _ = nonce.dep_encode(&mut key);
        let _ = service_id.dep_encode(&mut key);

        self.storage_raw().read_from_address(addr, key)
    }

    #[endpoint(submit_proof)]
    fn submit_proof(&self, job_id: ManagedBuffer, proof: ManagedBuffer) {
        let job_mapper = self.job_data(&job_id);
        require!(!job_mapper.is_empty(), "Job does not exist");

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

    #[view(get_job_data)]
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

    #[storage_mapper("identityRegistryAddress")]
    fn identity_registry_address(&self) -> SingleValueMapper<ManagedAddress>;
}
