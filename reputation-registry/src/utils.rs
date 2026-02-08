multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UtilsModule:
    common::cross_contract::CrossContractModule + crate::storage::StorageModule
{
    /// Cumulative moving average: new_score = (current * (n-1) + rating) / n
    fn calculate_new_score(&self, agent_nonce: u64, rating: BigUint) -> BigUint {
        let total_jobs = self.total_jobs(agent_nonce).update(|n| {
            *n += 1;
            *n
        });

        let current_score = self.reputation_score(agent_nonce).get();
        let total_big = BigUint::from(total_jobs);
        let prev_total = &total_big - 1u32;
        let weighted_score = current_score * prev_total;

        (weighted_score + rating) / total_big
    }

    /// Resolve agent owner from identity-registry via cross-contract BiDiMapper read.
    fn require_agent_owner(&self, agent_nonce: u64) -> ManagedAddress {
        let identity_addr = self.identity_contract_address().get();
        self.external_agents(identity_addr).get_value(&agent_nonce)
    }
}
