multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("escrowDeposited")]
    fn escrow_deposited_event(
        &self,
        #[indexed] job_id: &ManagedBuffer,
        #[indexed] employer: &ManagedAddress,
        amount: &NonZeroBigUint,
    );

    #[event("escrowReleased")]
    fn escrow_released_event(
        &self,
        #[indexed] job_id: &ManagedBuffer,
        #[indexed] receiver: &ManagedAddress,
        amount: &NonZeroBigUint,
    );

    #[event("escrowRefunded")]
    fn escrow_refunded_event(
        &self,
        #[indexed] job_id: &ManagedBuffer,
        #[indexed] employer: &ManagedAddress,
        amount: &NonZeroBigUint,
    );
}
