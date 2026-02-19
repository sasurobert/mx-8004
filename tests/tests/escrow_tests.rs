use escrow::storage::EscrowStatus;
use multiversx_sc::types::{BigUint, ManagedAddress, ManagedBuffer};
use multiversx_sc_scenario::api::StaticApi;
use mx_8004_tests::{constants::*, setup::EscrowTestState};

// ============================================
// 1. Deploy Escrow
// ============================================

#[test]
fn test_deploy_escrow() {
    let state = EscrowTestState::new();
    assert_ne!(state.escrow_sc, ManagedAddress::<StaticApi>::zero());
    assert_ne!(state.validation_sc, ManagedAddress::<StaticApi>::zero());
}

// ============================================
// 2. Deposit EGLD
// ============================================

#[test]
fn test_deposit_egld() {
    let mut state = EscrowTestState::new();

    state.deposit_egld(
        &EMPLOYER,
        b"job_egld_1",
        &AGENT_OWNER,
        b"poa_hash_123",
        1_000_000, // deadline far in the future
        500_000,   // amount
    );

    let escrow = state.query_escrow(b"job_egld_1");
    assert_eq!(escrow.employer, EMPLOYER.to_managed_address());
    assert_eq!(escrow.receiver, AGENT_OWNER.to_managed_address());
    assert_eq!(escrow.amount, BigUint::<StaticApi>::from(500_000u64));
    assert_eq!(
        escrow.poa_hash,
        ManagedBuffer::<StaticApi>::from(b"poa_hash_123")
    );
    assert_eq!(escrow.status, EscrowStatus::Active);
}

// ============================================
// 3. Deposit ESDT
// ============================================

#[test]
fn test_deposit_esdt() {
    let mut state = EscrowTestState::new();

    state.deposit_esdt(
        &EMPLOYER,
        b"job_esdt_1",
        &AGENT_OWNER,
        b"poa_hash_456",
        2_000_000,
        "USDC-abcdef",
        0,
        1_000,
    );

    let escrow = state.query_escrow(b"job_esdt_1");
    assert_eq!(escrow.employer, EMPLOYER.to_managed_address());
    assert_eq!(escrow.receiver, AGENT_OWNER.to_managed_address());
    assert_eq!(escrow.amount, BigUint::<StaticApi>::from(1_000u64));
    assert_eq!(escrow.status, EscrowStatus::Active);
}

// ============================================
// 4. Deposit Zero Amount → Error
// ============================================

#[test]
fn test_deposit_zero_amount() {
    let mut state = EscrowTestState::new();

    state.deposit_egld_expect_err(
        &EMPLOYER,
        b"job_zero",
        &AGENT_OWNER,
        b"poa_hash",
        1_000_000,
        0,
        "Deposit amount must be greater than zero",
    );
}

// ============================================
// 5. Deposit Duplicate Job → Error
// ============================================

#[test]
fn test_deposit_duplicate_job() {
    let mut state = EscrowTestState::new();

    state.deposit_egld(
        &EMPLOYER,
        b"job_dup",
        &AGENT_OWNER,
        b"poa_hash",
        1_000_000,
        500_000,
    );

    state.deposit_egld_expect_err(
        &EMPLOYER,
        b"job_dup",
        &AGENT_OWNER,
        b"poa_hash_2",
        2_000_000,
        500_000,
        "Escrow already exists for this job",
    );
}

// ============================================
// 6. Release Verified Job
// ============================================

#[test]
fn test_release_verified() {
    let mut state = EscrowTestState::new();

    // Register agent (needed for validation flow)
    state.register_agent(
        &AGENT_OWNER,
        b"EscrowAgent",
        b"https://agent.com",
        AGENT.to_address().as_bytes(), // Use AGENT address bytes as public key
        vec![],
        vec![],
    );

    // Init job in validation registry
    state.init_job(&EMPLOYER, b"job_release", 1, None);

    // Submit proof as AGENT (authorized via public key)
    state.submit_proof(&AGENT, b"job_release", b"proof_data");

    // Validation request + response (to get job to Verified status)
    state.validation_request(
        &AGENT_OWNER,
        b"job_release",
        &VALIDATOR,
        b"https://val.uri",
        b"req_hash_release",
    );
    state.validation_response(
        &VALIDATOR,
        b"req_hash_release",
        100,
        b"https://resp.uri",
        b"resp_hash",
        b"approved",
    );

    // Deposit into escrow
    state.deposit_egld(
        &EMPLOYER,
        b"job_release",
        &AGENT_OWNER,
        b"poa_hash",
        1_000_000,
        500_000,
    );

    // Employer releases funds
    state.release(&EMPLOYER, b"job_release");

    // Verify status changed to Released
    let escrow = state.query_escrow(b"job_release");
    assert_eq!(escrow.status, EscrowStatus::Released);
}

// ============================================
// 7. Release Not Verified → Error
// ============================================

#[test]
fn test_release_not_verified() {
    let mut state = EscrowTestState::new();

    // Register agent
    state.register_agent(
        &AGENT_OWNER,
        b"EscrowAgent",
        b"https://agent.com",
        AGENT.to_address().as_bytes(),
        vec![],
        vec![],
    );

    // Init job but do NOT go through validation
    state.init_job(&EMPLOYER, b"job_not_verified", 1, None);
    state.submit_proof(&AGENT, b"job_not_verified", b"proof_data");

    // Deposit into escrow
    state.deposit_egld(
        &EMPLOYER,
        b"job_not_verified",
        &AGENT_OWNER,
        b"poa_hash",
        1_000_000,
        500_000,
    );

    // Try to release → job not verified
    state.release_expect_err(
        &EMPLOYER,
        b"job_not_verified",
        "Job must be verified before release",
    );
}

// ============================================
// 8. Release Not Employer → Error
// ============================================

#[test]
fn test_release_not_employer() {
    let mut state = EscrowTestState::new();

    // Register agent
    state.register_agent(
        &AGENT_OWNER,
        b"EscrowAgent",
        b"https://agent.com",
        AGENT.to_address().as_bytes(),
        vec![],
        vec![],
    );

    // Full validation flow to get verified status
    state.init_job(&EMPLOYER, b"job_not_emp", 1, None);
    state.submit_proof(&AGENT, b"job_not_emp", b"proof");
    state.validation_request(
        &AGENT_OWNER,
        b"job_not_emp",
        &VALIDATOR,
        b"https://req.uri",
        b"req_hash_ne",
    );
    state.validation_response(
        &VALIDATOR,
        b"req_hash_ne",
        100,
        b"https://resp.uri",
        b"resp_hash",
        b"approved",
    );

    // Deposit
    state.deposit_egld(
        &EMPLOYER,
        b"job_not_emp",
        &AGENT_OWNER,
        b"poa_hash",
        1_000_000,
        500_000,
    );

    // Non-employer (CLIENT) tries to release → error
    state.release_expect_err(&CLIENT, b"job_not_emp", "Only the employer can call this");
}

// ============================================
// 9. Release Already Released → Error
// ============================================

#[test]
fn test_release_already_released() {
    let mut state = EscrowTestState::new();

    state.register_agent(
        &AGENT_OWNER,
        b"EscrowAgent",
        b"https://agent.com",
        AGENT.to_address().as_bytes(),
        vec![],
        vec![],
    );

    state.init_job(&EMPLOYER, b"job_double_rel", 1, None);
    state.submit_proof(&AGENT, b"job_double_rel", b"proof");
    state.validation_request(
        &AGENT_OWNER,
        b"job_double_rel",
        &VALIDATOR,
        b"https://req.uri",
        b"req_hash_dr",
    );
    state.validation_response(
        &VALIDATOR,
        b"req_hash_dr",
        100,
        b"https://resp.uri",
        b"resp_hash",
        b"approved",
    );

    state.deposit_egld(
        &EMPLOYER,
        b"job_double_rel",
        &AGENT_OWNER,
        b"poa_hash",
        1_000_000,
        500_000,
    );

    // First release succeeds
    state.release(&EMPLOYER, b"job_double_rel");

    // Second release → already settled
    state.release_expect_err(&EMPLOYER, b"job_double_rel", "Escrow already settled");
}

// ============================================
// 10. Refund After Deadline
// ============================================

#[test]
fn test_refund_after_deadline() {
    let mut state = EscrowTestState::new();

    // Set initial block timestamp to 100
    state.world.current_block().block_timestamp_seconds(100);

    state.deposit_egld(
        &EMPLOYER,
        b"job_refund",
        &AGENT_OWNER,
        b"poa_hash",
        200, // deadline at timestamp 200
        500_000,
    );

    // Advance time past deadline
    state.world.current_block().block_timestamp_seconds(201);

    // Anyone can refund (using CLIENT here to prove it)
    state.refund(&CLIENT, b"job_refund");

    // Verify status changed to Refunded
    let escrow = state.query_escrow(b"job_refund");
    assert_eq!(escrow.status, EscrowStatus::Refunded);
}

// ============================================
// 11. Refund Before Deadline → Error
// ============================================

#[test]
fn test_refund_before_deadline() {
    let mut state = EscrowTestState::new();

    state.world.current_block().block_timestamp_seconds(100);

    state.deposit_egld(
        &EMPLOYER,
        b"job_early_refund",
        &AGENT_OWNER,
        b"poa_hash",
        500, // deadline at 500
        500_000,
    );

    // Try refund at timestamp 100 (before 500 deadline)
    state.refund_expect_err(
        &EMPLOYER,
        b"job_early_refund",
        "Deadline has not passed yet",
    );
}

// ============================================
// 12. Refund Already Refunded → Error
// ============================================

#[test]
fn test_refund_already_refunded() {
    let mut state = EscrowTestState::new();

    state.world.current_block().block_timestamp_seconds(100);

    state.deposit_egld(
        &EMPLOYER,
        b"job_double_ref",
        &AGENT_OWNER,
        b"poa_hash",
        200,
        500_000,
    );

    state.world.current_block().block_timestamp_seconds(201);

    // First refund succeeds
    state.refund(&EMPLOYER, b"job_double_ref");

    // Second refund → already settled
    state.refund_expect_err(&EMPLOYER, b"job_double_ref", "Escrow already settled");
}

// ============================================
// 13. Release After Refund → Error
// ============================================

#[test]
fn test_release_after_refund() {
    let mut state = EscrowTestState::new();

    state.register_agent(
        &AGENT_OWNER,
        b"EscrowAgent",
        b"https://agent.com",
        AGENT.to_address().as_bytes(),
        vec![],
        vec![],
    );

    state.world.current_block().block_timestamp_seconds(100);

    state.init_job(&EMPLOYER, b"job_ref_then_rel", 1, None);
    state.submit_proof(&AGENT, b"job_ref_then_rel", b"proof");
    state.validation_request(
        &AGENT_OWNER,
        b"job_ref_then_rel",
        &VALIDATOR,
        b"https://req.uri",
        b"req_hash_rr",
    );
    state.validation_response(
        &VALIDATOR,
        b"req_hash_rr",
        100,
        b"https://resp.uri",
        b"resp_hash",
        b"approved",
    );

    state.deposit_egld(
        &EMPLOYER,
        b"job_ref_then_rel",
        &AGENT_OWNER,
        b"poa_hash",
        200,
        500_000,
    );

    // Refund first (deadline passed)
    state.world.current_block().block_timestamp_seconds(201);
    state.refund(&EMPLOYER, b"job_ref_then_rel");

    // Try release after refund → already settled
    state.release_expect_err(&EMPLOYER, b"job_ref_then_rel", "Escrow already settled");
}

// ============================================
// 14. Refund After Release → Error
// ============================================

#[test]
fn test_refund_after_release() {
    let mut state = EscrowTestState::new();

    state.register_agent(
        &AGENT_OWNER,
        b"EscrowAgent",
        b"https://agent.com",
        AGENT.to_address().as_bytes(),
        vec![],
        vec![],
    );

    state.world.current_block().block_timestamp_seconds(100);

    state.init_job(&EMPLOYER, b"job_rel_then_ref", 1, None);
    state.submit_proof(&AGENT, b"job_rel_then_ref", b"proof");
    state.validation_request(
        &AGENT_OWNER,
        b"job_rel_then_ref",
        &VALIDATOR,
        b"https://req.uri",
        b"req_hash_rr2",
    );
    state.validation_response(
        &VALIDATOR,
        b"req_hash_rr2",
        100,
        b"https://resp.uri",
        b"resp_hash",
        b"approved",
    );

    state.deposit_egld(
        &EMPLOYER,
        b"job_rel_then_ref",
        &AGENT_OWNER,
        b"poa_hash",
        200,
        500_000,
    );

    // Release first (job is verified)
    state.release(&EMPLOYER, b"job_rel_then_ref");

    // Advance past deadline and try refund → already settled
    state.world.current_block().block_timestamp_seconds(201);
    state.refund_expect_err(&CLIENT, b"job_rel_then_ref", "Escrow already settled");
}

// ============================================
// 15. Release Non-Existent Escrow → Error
// ============================================

#[test]
fn test_release_nonexistent() {
    let mut state = EscrowTestState::new();

    state.release_expect_err(&EMPLOYER, b"no_such_job", "Escrow not found for this job");
}

// ============================================
// 16. Refund Non-Existent Escrow → Error
// ============================================

#[test]
fn test_refund_nonexistent() {
    let mut state = EscrowTestState::new();

    state.refund_expect_err(&EMPLOYER, b"no_such_job", "Escrow not found for this job");
}

// ============================================
// 17. Full Lifecycle EGLD: Deposit → Verify → Release
// ============================================

#[test]
fn test_full_lifecycle_egld() {
    let mut state = EscrowTestState::new();

    // 1. Register agent
    state.register_agent(
        &AGENT_OWNER,
        b"LifecycleAgent",
        b"https://lifecycle.agent.com",
        AGENT.to_address().as_bytes(),
        vec![(b"type", b"escrow-test")],
        vec![],
    );

    // 2. Init job + validation flow
    state.init_job(&EMPLOYER, b"lifecycle_egld", 1, None);
    state.submit_proof(&AGENT, b"lifecycle_egld", b"proof_lc");
    state.validation_request(
        &AGENT_OWNER,
        b"lifecycle_egld",
        &VALIDATOR,
        b"https://val.uri",
        b"lc_hash",
    );
    state.validation_response(
        &VALIDATOR,
        b"lc_hash",
        95,
        b"https://resp.uri",
        b"lc_resp",
        b"approved",
    );

    // Verify job is verified
    assert!(state.query_is_job_verified(b"lifecycle_egld"));

    // 3. Deposit escrow
    state.deposit_egld(
        &EMPLOYER,
        b"lifecycle_egld",
        &AGENT_OWNER,
        b"poa_lifecycle",
        1_000_000,
        1_000_000, // 1M EGLD units
    );

    // 4. Verify escrow data
    let escrow = state.query_escrow(b"lifecycle_egld");
    assert_eq!(escrow.status, EscrowStatus::Active);
    assert_eq!(escrow.amount, BigUint::<StaticApi>::from(1_000_000u64));

    // 5. Release
    state.release(&EMPLOYER, b"lifecycle_egld");

    // 6. Verify final state
    let escrow = state.query_escrow(b"lifecycle_egld");
    assert_eq!(escrow.status, EscrowStatus::Released);
}

// ============================================
// 18. Full Lifecycle ESDT: Deposit → Verify → Release
// ============================================

#[test]
fn test_full_lifecycle_esdt() {
    let mut state = EscrowTestState::new();

    // 1. Register agent
    state.register_agent(
        &AGENT_OWNER,
        b"EsdtAgent",
        b"https://esdt.agent.com",
        AGENT.to_address().as_bytes(),
        vec![],
        vec![(1u32, 500u64, b"USDC-abcdef", 0u64)],
    );

    // 2. Init job + validation flow
    state.init_job(&EMPLOYER, b"lifecycle_esdt", 1, None);
    state.submit_proof(&AGENT, b"lifecycle_esdt", b"proof_esdt");
    state.validation_request(
        &AGENT_OWNER,
        b"lifecycle_esdt",
        &VALIDATOR,
        b"https://val.uri",
        b"esdt_hash",
    );
    state.validation_response(
        &VALIDATOR,
        b"esdt_hash",
        90,
        b"https://resp.uri",
        b"esdt_resp",
        b"approved",
    );

    // 3. Deposit ESDT into escrow
    state.deposit_esdt(
        &EMPLOYER,
        b"lifecycle_esdt",
        &AGENT_OWNER,
        b"poa_esdt",
        1_000_000,
        "USDC-abcdef",
        0,
        500,
    );

    // 4. Verify escrow
    let escrow = state.query_escrow(b"lifecycle_esdt");
    assert_eq!(escrow.status, EscrowStatus::Active);
    assert_eq!(escrow.amount, BigUint::<StaticApi>::from(500u64));

    // 5. Release
    state.release(&EMPLOYER, b"lifecycle_esdt");

    // 6. Verify final state
    let escrow = state.query_escrow(b"lifecycle_esdt");
    assert_eq!(escrow.status, EscrowStatus::Released);
}
