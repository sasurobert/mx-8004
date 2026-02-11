multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("escrow_deposited")]
    fn escrow_deposited_event(
        &self,
        #[indexed] job_id: &ManagedBuffer,
        #[indexed] employer: &ManagedAddress,
        amount: &NonZeroBigUint,
    );

    #[event("escrow_released")]
    fn escrow_released_event(
        &self,
        #[indexed] job_id: &ManagedBuffer,
        #[indexed] receiver: &ManagedAddress,
        amount: &NonZeroBigUint,
    );

    #[event("escrow_refunded")]
    fn escrow_refunded_event(
        &self,
        #[indexed] job_id: &ManagedBuffer,
        #[indexed] employer: &ManagedAddress,
        amount: &NonZeroBigUint,
    );
}
