use identity_registry::*;
use multiversx_sc::types::{BigUint, ManagedAddress, ManagedBuffer};
use multiversx_sc_scenario::rust_biguint;
use multiversx_sc_scenario::testing_framework::BlockchainStateWrapper;
use reputation_registry::*;
use validation_registry::{self, ValidationRegistry};

const ID_WASM: &str = "output/identity-registry.wasm";
const VAL_WASM: &str = "output/validation-registry.wasm";
const REP_WASM: &str = "output/reputation-registry.wasm";

#[test]
fn test_authenticity_loop() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_biguint!(0));
    let agent = b_mock.create_user_account(&rust_biguint!(0));
    let user = b_mock.create_user_account(&rust_biguint!(0));
    let oracle = b_mock.create_user_account(&rust_biguint!(0));

    // 1. Deploy Contracts
    let id_sc = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner),
        identity_registry::contract_obj,
        ID_WASM,
    );
    let val_sc = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner),
        validation_registry::contract_obj,
        VAL_WASM,
    );
    let rep_sc = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner),
        reputation_registry::contract_obj,
        REP_WASM,
    );

    // 2. Setup reputation registry
    b_mock
        .execute_tx(&owner, &rep_sc, &rust_biguint!(0), |sc| {
            sc.validation_contract_address()
                .set(ManagedAddress::from(val_sc.address_ref().clone()));
        })
        .assert_ok();

    // 3. Register Agent
    b_mock
        .execute_tx(&agent, &id_sc, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("ipfs://manifest"),
                ManagedBuffer::from("pubkey"),
            );
        })
        .assert_ok();

    // 3. Init Job (by User)
    b_mock
        .execute_tx(&user, &val_sc, &rust_biguint!(0), |sc| {
            sc.init_job(ManagedBuffer::from("job_123"), 1u64); // agent_nonce = 1
        })
        .assert_ok();

    // 4. Submit Proof (by Agent)
    b_mock
        .execute_tx(&agent, &val_sc, &rust_biguint!(0), |sc| {
            sc.submit_proof(
                ManagedBuffer::from("job_123"),
                ManagedBuffer::from("proof_hash"),
            );
        })
        .assert_ok();

    // 5. Verify Job (by Oracle)
    b_mock
        .execute_tx(&oracle, &val_sc, &rust_biguint!(0), |sc| {
            sc.verify_job(ManagedBuffer::from("job_123"));
        })
        .assert_ok();

    // 6. Authorize Feedback (by Agent)
    b_mock
        .execute_tx(&agent, &rep_sc, &rust_biguint!(0), |sc| {
            sc.authorize_feedback(
                ManagedBuffer::from("job_123"),
                ManagedAddress::from(user.clone()),
            );
        })
        .assert_ok();

    // 7. Submit Feedback (by User)
    b_mock
        .execute_tx(&user, &rep_sc, &rust_biguint!(0), |sc| {
            sc.submit_feedback(
                ManagedBuffer::from("job_123"),
                1u64,                       // agent_nonce
                rust_biguint!(5000).into(), // rating (5.000)
            );
        })
        .assert_ok();

    // 8. Verify result
    let _ = b_mock.execute_query(&rep_sc, |sc| {
        assert_eq!(sc.reputation_score(1u64).get(), BigUint::from(5000u64));
        assert_eq!(sc.total_jobs(1u64).get(), 1u64);
    });
}
