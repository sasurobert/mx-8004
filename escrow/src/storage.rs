multiversx_sc::imports!();
multiversx_sc::derive_imports!();

/// Escrow settlement status.
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub enum EscrowStatus {
    Active,
    Released,
    Refunded,
}

/// On-chain escrow record.
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct EscrowData<M: ManagedTypeApi> {
    /// Who deposited the funds
    pub employer: ManagedAddress<M>,
    /// Who receives on release (agent)
    pub receiver: ManagedAddress<M>,
    /// Payment details: token, nonce, amount
    pub payment: Payment<M>,
    /// Proof-of-Agreement hash
    pub poa_hash: ManagedBuffer<M>,
    /// Unix timestamp (seconds) of the block for the escrow deadline
    pub deadline: TimestampSeconds,
    /// Current state of the escrow
    pub status: EscrowStatus,
}

#[multiversx_sc::module]
pub trait StorageModule {
    #[view(get_escrow)]
    #[storage_mapper("escrowData")]
    fn escrow_data(&self, job_id: &ManagedBuffer) -> SingleValueMapper<EscrowData<Self::Api>>;

    #[view(get_validation_contract_address)]
    #[storage_mapper("validationContractAddress")]
    fn validation_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(get_identity_contract_address)]
    #[storage_mapper("identityContractAddress")]
    fn identity_contract_address(&self) -> SingleValueMapper<ManagedAddress>;
}
