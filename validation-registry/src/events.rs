multiversx_sc::imports!();

use crate::structs::JobStatus;

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("jobVerified")]
    fn job_verified_event(
        &self,
        #[indexed] job_id: ManagedBuffer,
        #[indexed] agent_nonce: u64,
        status: JobStatus,
    );
}
