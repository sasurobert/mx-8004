use multiversx_sc::types::{BigUint, ManagedAddress, ManagedBuffer};
use multiversx_sc_scenario::rust_biguint;
use multiversx_sc_scenario::testing_framework::BlockchainStateWrapper;
use reputation_registry::*;
use validation_registry::{self, ValidationRegistry};

const REP_WASM_PATH: &str = "output/reputation-registry.wasm";
const VAL_WASM_PATH: &str = "output/validation-registry.wasm";

#[test]
fn test_reputation_flow() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let user_addr = b_mock.create_user_account(&rust_biguint!(0));
    let _agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    // 1. Setup Validation Registry
    let val_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        validation_registry::contract_obj,
        VAL_WASM_PATH,
    );

    // 2. Setup Reputation Registry
    let rep_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        reputation_registry::contract_obj,
        REP_WASM_PATH,
    );

    // 3. Initialize Reputation Registry with Validation address
    let val_addr = val_wrapper.address_ref().clone();
    b_mock
        .execute_tx(&owner_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.validation_contract_address()
                .set(ManagedAddress::from(val_addr));
        })
        .assert_ok();

    // 4. Submit feedback for UNVERIFIED job -> Should FAIL
    b_mock
        .execute_tx(&user_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.submit_feedback(ManagedBuffer::from("job_1"), 1u64, BigUint::from(5u64));
        })
        .assert_user_error("Job not found or not initialized");

    // 5. Initialize Job and Authorize Feedback
    b_mock
        .execute_tx(&user_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.init_job(ManagedBuffer::from("job_1"), 1u64);
        })
        .assert_ok();

    b_mock
        .execute_tx(&owner_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.authorize_feedback(ManagedBuffer::from("job_1"), user_addr.clone().into());
        })
        .assert_ok();

    // 6. Verify job in Validation Registry
    b_mock
        .execute_tx(&owner_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.verify_job(ManagedBuffer::from("job_1"));
        })
        .assert_ok();

    // 7. Submit feedback for VERIFIED job -> Should PASS
    b_mock
        .execute_tx(&user_addr, &rep_wrapper, &rust_biguint!(0), |sc| {
            sc.submit_feedback(ManagedBuffer::from("job_1"), 1u64, BigUint::from(5u64));
        })
        .assert_ok();

    // 7. Check reputation
    let _ = b_mock.execute_query(&rep_wrapper, |sc| {
        let score = sc.reputation_score(1u64).get();
        assert_eq!(score, BigUint::from(5u64)); // Simplified for test
    });
}
