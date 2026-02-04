use identity_registry::*;
use multiversx_sc::types::{EsdtLocalRole, ManagedAddress, ManagedBuffer, TokenIdentifier};
use multiversx_sc_scenario::imports::OptionalValue;
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

    // 1. Issue Token (simulated)
    b_mock
        .execute_tx(&owner_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.agent_token_id()
                .set_token_id(TokenIdentifier::from("AGENT-123456"));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        id_wrapper.address_ref(),
        b"AGENT-123456",
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftUpdateAttributes],
    );

    // 2. Register Agent
    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("ipfs://hash"),
                ManagedBuffer::from("public_key_hex"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    // 3. Verify Agent Details
    let _ = b_mock.execute_query(&id_wrapper, |sc| {
        let agent = sc.get_agent(1);
        assert_eq!(agent.name, ManagedBuffer::from("Moltbot-01"));
        assert_eq!(agent.uri, ManagedBuffer::from("ipfs://hash"));
        assert_eq!(agent.public_key, ManagedBuffer::from("public_key_hex"));
        assert_eq!(agent.owner, ManagedAddress::from(agent_addr.clone()));

        assert_eq!(sc.get_agent_id(ManagedAddress::from(agent_addr.clone())), 1);
    });
}

#[test]
fn test_update_agent() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        WASM_PATH,
    );

    // 1. Setup (Issue + Roles + Register)
    b_mock
        .execute_tx(&owner_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.agent_token_id()
                .set_token_id(TokenIdentifier::from("AGENT-123456"));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        id_wrapper.address_ref(),
        b"AGENT-123456",
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftUpdateAttributes],
    );

    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("ipfs://hash"),
                ManagedBuffer::from("public_key_hex"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    // 2. Update (Agent should still have the NFT)
    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.update_agent(
                1,
                ManagedBuffer::from("ipfs://new_hash"),
                ManagedBuffer::from("new_public_key_hex"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    // 3. Verify
    let _ = b_mock.execute_query(&id_wrapper, |sc| {
        let agent = sc.get_agent(1);
        assert_eq!(agent.uri, ManagedBuffer::from("ipfs://new_hash"));
        assert_eq!(agent.public_key, ManagedBuffer::from("new_public_key_hex"));
    });
}

use multiversx_sc_scenario::*;

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();
    blockchain.register_contract(
        "file:../identity-registry/output/identity-registry.wasm",
        identity_registry::ContractBuilder,
    );
    blockchain
}

#[test]
fn identity_full_flow_scen() {
    world().run("../scenarios/identity_full_flow.scen.json");
}

#[test]
fn simple_scen() {
    world().run("../scenarios/simple.scen.json");
}
