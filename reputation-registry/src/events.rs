multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("reputationUpdated")]
    fn reputation_updated_event(&self, #[indexed] agent_nonce: u64, new_score: BigUint);
}
