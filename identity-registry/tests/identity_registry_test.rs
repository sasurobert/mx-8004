use identity_registry::*;
use multiversx_sc::types::{ManagedAddress, ManagedBuffer};
use multiversx_sc_scenario::rust_biguint;
use multiversx_sc_scenario::testing_framework::BlockchainStateWrapper;

const WASM_PATH: &str = "output/identity-registry.wasm";

#[test]
fn test_register_agent() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        WASM_PATH,
    );

    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("ipfs://hash"),
                ManagedBuffer::from("public_key_hex"),
            );
        })
        .assert_ok();

    let _ = b_mock.execute_query(&id_wrapper, |sc| {
        let agent = sc.get_agent(1);
        assert_eq!(agent.name, ManagedBuffer::from("Moltbot-01"));
        assert_eq!(agent.uri, ManagedBuffer::from("ipfs://hash"));
        assert_eq!(agent.public_key, ManagedBuffer::from("public_key_hex"));
        // Compare with agent_addr directly
        assert_eq!(agent.owner, ManagedAddress::from(agent_addr.clone()));
    });
}
