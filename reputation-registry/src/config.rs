multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ConfigModule:
    common::cross_contract::CrossContractModule + crate::storage::StorageModule
{
    #[only_owner]
    #[endpoint(setIdentityContractAddress)]
    fn set_identity_contract_address(&self, address: ManagedAddress) {
        self.identity_contract_address().set(&address);
    }

    #[only_owner]
    #[endpoint(setValidationContractAddress)]
    fn set_validation_contract_address(&self, address: ManagedAddress) {
        self.validation_contract_address().set(&address);
    }
}
