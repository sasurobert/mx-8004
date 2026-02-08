use identity_registry::{self, IdentityRegistry};
use multiversx_sc::types::{EsdtLocalRole, ManagedAddress, ManagedBuffer, TokenIdentifier};
use multiversx_sc_scenario::imports::OptionalValue;
use multiversx_sc_scenario::rust_biguint;
use multiversx_sc_scenario::testing_framework::BlockchainStateWrapper;
use reputation_registry::*;
use validation_registry::{self, ValidationRegistry};

const REP_WASM_PATH: &str = "output/reputation-registry.wasm";
const VAL_WASM_PATH: &str = "output/validation-registry.wasm";
const ID_WASM_PATH: &str = "output/identity-registry.wasm";

#[test]
fn test_unauthorized_feedback_authorization() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let user_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));
    let attacker_addr = b_mock.create_user_account(&rust_biguint!(0));

    // 1. Setup Validation Registry
    let val_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        validation_registry::contract_obj,
        VAL_WASM_PATH,
    );

    // 1.5 Setup Identity Registry
    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        ID_WASM_PATH,
    );

    // 2. Setup Reputation Registry
    let rep_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        reputation_registry::contract_obj,
        REP_WASM_PATH,
    );

    // 3. Initialize Reputation Registry with Validation & Identity addresses
    let val_addr = val_wrapper.address_ref().clone();
    let id_addr = id_wrapper.address_ref().clone();
    b_mock
        .execute_tx(&owner_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.validation_contract_address()
                .set(ManagedAddress::from(val_addr));
            sc.identity_contract_address()
                .set(ManagedAddress::from(id_addr));
        })
        .assert_ok();

    // 3.1 Setup Identity Registry Token
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

    // 3.2 Register Agent (Nonce 1)
    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("ipfs://manifest"),
                ManagedBuffer::from("pubkey"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    // 4. Initialize Job (by User) for Agent 1
    b_mock
        .execute_tx(&user_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.init_job(ManagedBuffer::from("job_1"), 1u64);
        })
        .assert_ok();

    // 5. Attacker tries to authorize feedback for Agent 1's job
    b_mock
        .execute_tx(&attacker_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.authorize_feedback(ManagedBuffer::from("job_1"), user_addr.clone().into());
        })
        .assert_user_error("Only the agent owner can authorize feedback");
}
