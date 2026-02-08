#![allow(clippy::too_many_arguments)]

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ExternalStorageModule {
    #[storage_mapper_from_address("agentOwner")]
    fn identity_agent_owner(
        &self,
        address: ManagedAddress,
        nonce: u64,
    ) -> SingleValueMapper<ManagedAddress, ManagedAddress>;

    #[storage_mapper_from_address("agentServicePrice")]
    fn identity_agent_service_price(
        &self,
        address: ManagedAddress,
        nonce: u64,
        service_id: &ManagedBuffer,
    ) -> SingleValueMapper<BigUint, ManagedAddress>;

    #[storage_mapper_from_address("agentServicePaymentToken")]
    fn identity_agent_service_payment_token(
        &self,
        address: ManagedAddress,
        nonce: u64,
        service_id: &ManagedBuffer,
    ) -> SingleValueMapper<EgldOrEsdtTokenIdentifier, ManagedAddress>;

    #[storage_mapper_from_address("agentServicePaymentNonce")]
    fn identity_agent_service_payment_nonce(
        &self,
        address: ManagedAddress,
        nonce: u64,
        service_id: &ManagedBuffer,
    ) -> SingleValueMapper<u64, ManagedAddress>;
}
