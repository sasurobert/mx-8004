use multiversx_sc::types::ManagedBuffer;
use multiversx_sc_scenario::rust_biguint;
use multiversx_sc_scenario::testing_framework::BlockchainStateWrapper;
use validation_registry::*;

const WASM_PATH: &str = "output/validation-registry.wasm";

#[test]
fn test_validation_flow() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));
    let oracle_addr = b_mock.create_user_account(&rust_biguint!(0));

    let val_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        validation_registry::contract_obj,
        WASM_PATH,
    );

    // 0. Init Job
    b_mock
        .execute_tx(&owner_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.init_job(ManagedBuffer::from("job_1"), 1u64); // mock agent_nonce = 1
        })
        .assert_ok();

    // 1. Submit Proof (by Agent)
    b_mock
        .execute_tx(&agent_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.submit_proof(
                ManagedBuffer::from("job_1"),       // job_id
                ManagedBuffer::from("result_hash"), // proof
            );
        })
        .assert_ok();

    // 2. Initial state: Pending
    let _ = b_mock.execute_query(&val_wrapper, |sc| {
        assert!(!sc.is_job_verified(ManagedBuffer::from("job_1")));
    });

    // 3. Verify Job (by Oracle)
    b_mock
        .execute_tx(&oracle_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.verify_job(ManagedBuffer::from("job_1"));
        })
        .assert_ok();

    // 4. Final state: Verified
    let _ = b_mock.execute_query(&val_wrapper, |sc| {
        assert!(sc.is_job_verified(ManagedBuffer::from("job_1")));
    });
}

use multiversx_sc_scenario::*;

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();
    blockchain.register_contract(
        "file:../identity-registry/output/identity-registry.wasm",
        identity_registry::ContractBuilder,
    );
    blockchain.register_contract(
        "file:../validation-registry/output/validation-registry.wasm",
        validation_registry::ContractBuilder,
    );
    blockchain
}

#[test]
fn validation_full_flow_scen() {
    world().run("../scenarios/validation_full_flow.scen.json");
}

#[test]
fn init_payment_verified_flow_scen() {
    world().run("../scenarios/init_payment_verified_flow.scen.json");
}
