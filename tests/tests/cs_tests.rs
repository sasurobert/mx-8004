// Chain-simulator E2E tests — feature-gated behind `chain-simulator-tests`.
// Run with: cargo test -p mx-8004-tests --features chain-simulator-tests -- --test-threads=1
//
// These tests require a running MultiversX chain simulator on http://localhost:8085

use mx_8004_tests::interact::CsInteract;
use serial_test::serial;

#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_deploy_all_cs() {
    let _ = env_logger::try_init();
    let interact = CsInteract::new().await;

    assert!(
        !interact.identity_addr.to_bech32_string().is_empty(),
        "Identity address should be non-empty"
    );
    assert!(
        !interact.validation_addr.to_bech32_string().is_empty(),
        "Validation address should be non-empty"
    );
    assert!(
        !interact.reputation_addr.to_bech32_string().is_empty(),
        "Reputation address should be non-empty"
    );
    assert!(
        !interact.agent_token_id.is_empty(),
        "Agent token ID should be set"
    );
    println!(
        "All contracts deployed successfully. Token: {}",
        interact.agent_token_id
    );
}

#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_register_agent_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    let bob = interact.agent_owner.clone();
    interact
        .register_agent(
            &bob,
            b"TestAgent",
            b"https://agent.example.com",
            b"pubkey123",
        )
        .await;

    let token_id = interact.query_agent_token_id().await;
    assert!(
        !token_id.is_empty(),
        "Token ID should exist after registration"
    );
    println!("Agent registered and confirmed with token: {token_id}");
}

#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_job_lifecycle_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    let bob = interact.agent_owner.clone();
    interact
        .register_agent(
            &bob,
            b"TestAgent",
            b"https://agent.example.com",
            b"pubkey123",
        )
        .await;

    let agent_nonce = 1u64;
    let carol = interact.client.clone();
    let worker = interact.worker.clone();

    interact.init_job(&carol, b"job-001", agent_nonce).await;
    interact
        .submit_proof(&worker, b"job-001", b"proof-data-hash")
        .await;

    // Use validation_request + validation_response flow
    let validator = interact.owner.clone();
    interact
        .validation_request(&bob, b"job-001", &validator, b"req-uri", b"req-hash")
        .await;
    interact
        .validation_response(
            &validator,
            b"req-hash",
            90,
            b"resp-uri",
            b"resp-hash",
            b"quality",
        )
        .await;

    let verified = interact.query_is_job_verified(b"job-001").await;
    assert!(verified, "Job should be verified");
    println!("Full job lifecycle (init -> proof -> validate) passed");
}

/// Full lifecycle including reputation feedback.
///
/// This test is ignored by default because the reputation-registry uses
/// `storage_mapper_from_address` (ManagedStorageReadFromAddress VM hook)
/// to read job data from the validation-registry. This VM hook only works
/// when both contracts are deployed in the same shard. On the chain simulator
/// with 3 shards, contracts deployed at different nonces land in different
/// shards non-deterministically, causing "Job not found" errors.
///
/// The full lifecycle (including feedback) is covered by the 32 scenario tests.
///
/// To run: cargo test -p mx-8004-tests --features chain-simulator-tests -- --ignored --test-threads=1
#[tokio::test]
#[serial]
#[ignore = "Requires same-shard deployment; storage_mapper_from_address is shard-local"]
async fn test_full_lifecycle_with_feedback_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    let bob = interact.agent_owner.clone();
    interact
        .register_agent(
            &bob,
            b"TestAgent",
            b"https://agent.example.com",
            b"pubkey123",
        )
        .await;

    let agent_nonce = 1u64;
    let carol = interact.client.clone();
    let worker = interact.worker.clone();

    // Job lifecycle
    interact.init_job(&carol, b"job-001", agent_nonce).await;
    interact
        .submit_proof(&worker, b"job-001", b"proof-data-hash")
        .await;

    // Validation flow
    let validator = interact.owner.clone();
    interact
        .validation_request(&bob, b"job-001", &validator, b"req-uri", b"req-hash")
        .await;
    interact
        .validation_response(
            &validator,
            b"req-hash",
            90,
            b"resp-uri",
            b"resp-hash",
            b"quality",
        )
        .await;

    // Reputation (cross-contract storage reads from validation)
    interact
        .give_feedback_simple(&carol, b"job-001", agent_nonce, 85)
        .await;
    interact
        .append_response(&bob, b"job-001", b"https://response.example.com/result")
        .await;

    let score = interact.query_reputation_score(agent_nonce).await;
    assert!(
        score > multiversx_sc_scenario::imports::RustBigUint::from(0u64),
        "Score should be > 0"
    );

    let total = interact.query_total_jobs(agent_nonce).await;
    assert_eq!(total, 1, "Should have 1 completed job");

    let has_feedback = interact.query_has_given_feedback(b"job-001").await;
    assert!(has_feedback, "Feedback should be recorded");

    println!("Full lifecycle with feedback passed!");
}

// ── Error-path tests ──

#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_duplicate_agent_registration_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    let bob = interact.agent_owner.clone();
    interact
        .register_agent(
            &bob,
            b"TestAgent",
            b"https://agent.example.com",
            b"pubkey123",
        )
        .await;

    // Second registration from same address should fail
    interact
        .register_agent_expect_err(
            &bob,
            b"TestAgent2",
            b"https://agent2.example.com",
            b"pubkey456",
            4,
            "Agent already registered for this address",
        )
        .await;

    println!("Duplicate agent registration correctly rejected");
}

#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_issue_token_already_issued_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    // Token is already issued during CsInteract::new(), second issuance should fail
    interact
        .issue_token_expect_err(4, "Token already issued")
        .await;

    println!("Duplicate token issuance correctly rejected");
}

#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_validation_request_not_owner_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    let bob = interact.agent_owner.clone();
    interact
        .register_agent(
            &bob,
            b"TestAgent",
            b"https://agent.example.com",
            b"pubkey123",
        )
        .await;

    let carol = interact.client.clone();
    interact.init_job(&carol, b"job-001", 1u64).await;

    let worker = interact.worker.clone();
    interact
        .submit_proof(&worker, b"job-001", b"proof-data")
        .await;

    // Non-owner (carol) tries to make a validation request — should fail
    interact
        .validation_request_expect_err(
            &carol,
            b"job-001",
            b"req-uri",
            b"req-hash",
            4,
            "Only the agent owner can request validation",
        )
        .await;

    println!("Non-owner validation request correctly rejected");
}

#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_submit_proof_nonexistent_job_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    let worker = interact.worker.clone();
    interact
        .submit_proof_expect_err(
            &worker,
            b"nonexistent-job",
            b"proof-data",
            4,
            "Job not found",
        )
        .await;

    println!("Proof for nonexistent job correctly rejected");
}

#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_init_job_duplicate_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    let bob = interact.agent_owner.clone();
    interact
        .register_agent(
            &bob,
            b"TestAgent",
            b"https://agent.example.com",
            b"pubkey123",
        )
        .await;

    let carol = interact.client.clone();
    interact.init_job(&carol, b"job-001", 1u64).await;

    // Same job_id again should fail
    interact
        .init_job_expect_err(&carol, b"job-001", 1u64, 4, "Job already initialized")
        .await;

    println!("Duplicate job init correctly rejected");
}

/// Test: register agent with a free service (price=0, service_id=1),
/// then init_job with that service_id but NO payment → should succeed.
#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_init_job_free_service_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    let bob = interact.agent_owner.clone();
    // Register agent with service_id=1, price=0, EGLD, nonce=0
    interact
        .register_agent_with_meta(
            &bob,
            b"FreeBot",
            b"https://free.example.com",
            b"pubkey123",
            &[],
            &[(1, 0, b"EGLD-000000", 0)], // free service
        )
        .await;

    let carol = interact.client.clone();
    // Init job with service_id=1 (free) and no payment → should succeed
    interact
        .init_job_with_free_service(&carol, b"free-job-001", 1u64, 1)
        .await;

    println!("Free service job init succeeded without payment");
}

/// Test: register agent with a paid service (1 EGLD, service_id=1),
/// then init_job with that service_id but NO payment → ERR_INSUFFICIENT_PAYMENT.
#[tokio::test]
#[cfg_attr(not(feature = "chain-simulator-tests"), ignore)]
#[serial]
async fn test_init_job_no_payment_for_paid_service_cs() {
    let _ = env_logger::try_init();
    let mut interact = CsInteract::new().await;

    let bob = interact.agent_owner.clone();
    // Register agent with service_id=1, price=1 EGLD
    interact
        .register_agent_with_meta(
            &bob,
            b"PaidBot",
            b"https://paid.example.com",
            b"pubkey123",
            &[],
            &[(1, 1_000_000_000_000_000_000, b"EGLD-000000", 0)], // 1 EGLD
        )
        .await;

    let carol = interact.client.clone();
    // Init job with service_id=1 but NO payment → should fail
    interact
        .init_job_with_free_service_expect_err(
            &carol,
            b"no-pay-job-001",
            1u64,
            1,
            4,
            "Insufficient payment",
        )
        .await;

    println!("No-payment for paid service correctly rejected");
}
