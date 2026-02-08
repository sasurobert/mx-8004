multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ConfigModule:
    common::cross_contract::CrossContractModule + crate::storage::ExternalStorageModule
{
    #[only_owner]
    #[endpoint(setIdentityRegistryAddress)]
    fn set_identity_registry_address(&self, address: ManagedAddress) {
        self.identity_registry_address().set(&address);
    }
}
