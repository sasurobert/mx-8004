use common::structs::JobStatus;
use multiversx_sc::proxy_imports::OptionalValue;
use multiversx_sc::types::{BigUint, ManagedAddress, ManagedBuffer};
use multiversx_sc_scenario::api::StaticApi;
use mx_8004_tests::{constants::*, setup::AgentTestState};

// ============================================
// 1. Deploy
// ============================================

#[test]
fn test_deploy_all_contracts() {
    let state = AgentTestState::new();
    // All 3 contracts deployed in new() — addresses are non-zero
    assert_ne!(
        state.identity_sc,
        multiversx_sc::types::ManagedAddress::<StaticApi>::zero()
    );
    assert_ne!(
        state.validation_sc,
        multiversx_sc::types::ManagedAddress::<StaticApi>::zero()
    );
    assert_ne!(
        state.reputation_sc,
        multiversx_sc::types::ManagedAddress::<StaticApi>::zero()
    );
}

// ============================================
// 2. Register Agent
// ============================================

#[test]
fn test_register_agent() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1")],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    // Verify agent details stored
    let details = state.query_agent_details(1);
    assert_eq!(details.name, ManagedBuffer::<StaticApi>::from(b"TestAgent"));
    assert_eq!(
        details.public_key,
        ManagedBuffer::<StaticApi>::from(b"pubkey123")
    );

    // Verify owner
    let owner = state.query_agent_owner(1);
    assert_eq!(owner, AGENT_OWNER.to_managed_address());

    // Verify metadata
    let meta = state.query_metadata(1, b"key1");
    assert!(meta.is_some());

    // Verify service config
    let svc = state.query_service_config(1, 1);
    assert!(svc.is_some());
}

// ============================================
// 3. Register Agent Duplicate
// ============================================

#[test]
fn test_register_agent_duplicate() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.register_agent_expect_err(
        &AGENT_OWNER,
        b"TestAgent2",
        b"https://agent2.example.com",
        b"pubkey456",
        "Agent already registered for this address",
    );
}

// ============================================
// 4. Update Agent (requires NFT transfer + Ed25519 sig)
// ============================================

// updateAgent requires Ed25519 signature verification at VM level.
// We test the error paths (wrong NFT owner) here. Full Ed25519 flow needs chain-simulator.

#[test]
fn test_update_agent_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    // CLIENT tries to update agent 1 (owned by AGENT_OWNER) -> error
    // CLIENT doesn't have the NFT, so we need to give them one to even try
    // Instead, test with wrong NFT token by trying from a non-owner
    state.update_agent_expect_err(
        &CLIENT,
        1,
        b"NewName",
        b"https://new.uri",
        b"newpubkey",
        "insufficient funds",
    );
}

// ============================================
// 5. Set Metadata
// ============================================

#[test]
fn test_set_metadata() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.set_metadata(&AGENT_OWNER, 1, vec![(b"desc", b"A cool agent")]);

    let meta = state.query_metadata(1, b"desc");
    assert!(meta.is_some());
}

// ============================================
// 6. Remove Metadata
// ============================================

#[test]
fn test_remove_metadata() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1")],
        vec![],
    );

    // Confirm exists
    let meta = state.query_metadata(1, b"key1");
    assert!(meta.is_some());

    // Remove
    state.remove_metadata(&AGENT_OWNER, 1, vec![b"key1"]);

    // Confirm gone
    let meta = state.query_metadata(1, b"key1");
    assert!(meta.is_none());
}

// ============================================
// 7. Set Service Configs
// ============================================

#[test]
fn test_set_service_configs() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.set_service_configs(&AGENT_OWNER, 1, vec![(42u32, 500u64, b"USDC-abcdef", 0u64)]);

    let svc = state.query_service_config(1, 42);
    assert!(svc.is_some());
}

// ============================================
// 8. Remove Service Configs
// ============================================

#[test]
fn test_remove_service_configs() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    // Confirm exists
    let svc = state.query_service_config(1, 1);
    assert!(svc.is_some());

    // Remove
    state.remove_service_configs(&AGENT_OWNER, 1, vec![1]);

    // Confirm gone
    let svc = state.query_service_config(1, 1);
    assert!(svc.is_none());
}

// ============================================
// 9. Init Job with Payment
// ============================================

#[test]
fn test_init_job_with_payment() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    state.init_job_with_payment(
        &CLIENT,
        b"job1",
        1, // agent_nonce
        1, // service_id
        "USDC-abcdef",
        0,
        100, // amount matching service price
    );

    let job = state.query_job_data(b"job1");
    assert!(job.is_some());
    if let OptionalValue::Some(data) = job {
        assert_eq!(data.status, JobStatus::New);
        assert_eq!(data.agent_nonce, 1);
    }
}

// ============================================
// 10. Init Job (no service, free)
// ============================================

#[test]
fn test_init_job_no_service() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_free", 1, None);

    let job = state.query_job_data(b"job_free");
    assert!(job.is_some());
}

// ============================================
// 11. Init Job Invalid Payment
// ============================================

#[test]
fn test_init_job_invalid_payment() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    // Wrong amount (too low)
    state.init_job_with_payment_expect_err(
        &CLIENT,
        b"job_bad",
        1,
        1,
        "USDC-abcdef",
        0,
        50, // insufficient
        "Insufficient payment",
    );
}

// ============================================
// 12. Submit Proof
// ============================================

#[test]
fn test_submit_proof() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_proof", 1, None);
    // submit_proof is open — anyone (e.g. WORKER agent) can call it
    state.submit_proof(&WORKER, b"job_proof", b"proof_data_here");

    let job = state.query_job_data(b"job_proof");
    if let OptionalValue::Some(data) = job {
        assert_eq!(data.status, JobStatus::Pending);
        assert_eq!(
            data.proof,
            ManagedBuffer::<StaticApi>::from(b"proof_data_here")
        );
    }
}

// ============================================
// 13. Validation Request
// ============================================

#[test]
fn test_validation_request() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_vr", 1, None);
    state.submit_proof(&WORKER, b"job_vr", b"proof123");

    // Agent owner requests validation
    state.validation_request(
        &AGENT_OWNER,
        b"job_vr",
        &VALIDATOR,
        b"https://request.uri",
        b"req_hash_001",
    );

    let job = state.query_job_data(b"job_vr");
    if let OptionalValue::Some(data) = job {
        assert_eq!(data.status, JobStatus::ValidationRequested);
    }
}

// ============================================
// 14. Validation Request — Not Agent Owner
// ============================================

#[test]
fn test_validation_request_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_notowner", 1, None);
    state.submit_proof(&WORKER, b"job_notowner", b"proof");

    // CLIENT (not agent owner) tries to request validation
    state.validation_request_expect_err(
        &CLIENT,
        b"job_notowner",
        &VALIDATOR,
        b"https://request.uri",
        b"req_hash_err",
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 14b. Validation Response
// ============================================

#[test]
fn test_validation_response() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_resp", 1, None);
    state.submit_proof(&WORKER, b"job_resp", b"proof123");
    state.validation_request(
        &AGENT_OWNER,
        b"job_resp",
        &VALIDATOR,
        b"https://request.uri",
        b"req_hash_resp",
    );

    // Validator responds
    state.validation_response(
        &VALIDATOR,
        b"req_hash_resp",
        85,
        b"https://response.uri",
        b"resp_hash_001",
        b"quality",
    );
}

// ============================================
// 14c. Validation Response — Not Validator
// ============================================

#[test]
fn test_validation_response_not_validator() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_nv", 1, None);
    state.submit_proof(&WORKER, b"job_nv", b"proof");
    state.validation_request(
        &AGENT_OWNER,
        b"job_nv",
        &VALIDATOR,
        b"https://request.uri",
        b"req_hash_nv",
    );

    // CLIENT (not the designated validator) tries to respond
    state.validation_response_expect_err(
        &CLIENT,
        b"req_hash_nv",
        80,
        b"https://response.uri",
        b"resp_hash",
        b"tag",
        "Only the designated validator can respond",
    );
}

// ============================================
// 15. Clean Old Jobs
// ============================================

#[test]
fn test_clean_old_jobs() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    // Set block timestamp to 0
    state.world.current_block().block_timestamp_millis(0);
    state.init_job(&CLIENT, b"job_old", 1, None);

    // Advance time by 4 days (> 3 days threshold)
    let four_days_ms: u64 = 4 * 24 * 60 * 60 * 1000;
    state
        .world
        .current_block()
        .block_timestamp_millis(four_days_ms);

    state.clean_old_jobs(vec![b"job_old"]);

    // Job should be cleaned
    let job = state.query_job_data(b"job_old");
    assert!(job.is_none());
}

// ============================================
// 16. Full Feedback Flow
// ============================================

#[test]
fn test_full_feedback_flow() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_fb", 1, None);
    state.submit_proof(&WORKER, b"job_fb", b"proof");

    // ERC-8004: employer (CLIENT) submits feedback directly — no authorization needed
    state.give_feedback_simple(&CLIENT, b"job_fb", 1, 80);

    // Verify reputation updated
    let score = state.query_reputation_score(1);
    assert_eq!(score, BigUint::<StaticApi>::from(80u64));

    let total = state.query_total_jobs(1);
    assert_eq!(total, 1u64);

    assert!(state.query_has_given_feedback(b"job_fb"));
}

// ============================================
// 17. Feedback Guards
// ============================================

#[test]
fn test_feedback_guards() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_guard", 1, None);
    state.submit_proof(&WORKER, b"job_guard", b"proof");

    // Non-employer tries to submit feedback -> error
    state.give_feedback_simple_expect_err(
        &WORKER,
        b"job_guard",
        1,
        80,
        "Only the employer can provide feedback",
    );

    // Employer submits feedback (no authorize needed in ERC-8004)
    state.give_feedback_simple(&CLIENT, b"job_guard", 1, 90);

    // Duplicate feedback -> error
    state.give_feedback_simple_expect_err(
        &CLIENT,
        b"job_guard",
        1,
        90,
        "Feedback already provided for this job",
    );
}

// ============================================
// 18. Full Lifecycle E2E
// ============================================

#[test]
fn test_full_lifecycle() {
    let mut state = AgentTestState::new();

    // 1. Register agent with metadata and service config
    state.register_agent(
        &AGENT_OWNER,
        b"FullAgent",
        b"https://full.agent.com",
        b"pubkey_full",
        vec![(b"category", b"AI"), (b"version", b"1.0")],
        vec![(1u32, 200u64, b"USDC-abcdef", 0u64)],
    );

    // 2. Init job with payment
    state.init_job_with_payment(&CLIENT, b"lifecycle_job", 1, 1, "USDC-abcdef", 0, 200);

    // 3. Submit proof (WORKER = agent)
    state.submit_proof(&WORKER, b"lifecycle_job", b"proof_lifecycle");

    // 4. Validation request (agent owner)
    state.validation_request(
        &AGENT_OWNER,
        b"lifecycle_job",
        &VALIDATOR,
        b"https://val.uri",
        b"lifecycle_hash",
    );

    // 5. Validation response (validator)
    state.validation_response(
        &VALIDATOR,
        b"lifecycle_hash",
        90,
        b"https://resp.uri",
        b"resp_hash",
        b"approved",
    );

    // 6. Submit feedback (employer, no authorize needed)
    state.give_feedback_simple(&CLIENT, b"lifecycle_job", 1, 95);
    assert_eq!(
        state.query_reputation_score(1),
        BigUint::<StaticApi>::from(95u64)
    );

    // 7. Append response (permissionless in ERC-8004)
    state.append_response(&AGENT_OWNER, b"lifecycle_job", b"https://response.uri");
    let response = state.query_agent_response(b"lifecycle_job");
    assert_eq!(
        response,
        ManagedBuffer::<StaticApi>::from(b"https://response.uri")
    );
}

// ============================================
// 19. Upgrade All Contracts
// ============================================

#[test]
fn test_upgrade_all_contracts() {
    let mut state = AgentTestState::new();

    // Register agent before upgrade to verify state persists
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1")],
        vec![],
    );

    // Upgrade all 3
    state.upgrade_identity();
    state.upgrade_validation();
    state.upgrade_reputation();

    // Verify state persists after upgrade
    let details = state.query_agent_details(1);
    assert_eq!(details.name, ManagedBuffer::<StaticApi>::from(b"TestAgent"));
    let owner = state.query_agent_owner(1);
    assert_eq!(owner, AGENT_OWNER.to_managed_address());
}

// ============================================
// 20. Issue Token (already issued error)
// ============================================

#[test]
fn test_issue_token_already_issued() {
    let mut state = AgentTestState::new();
    // Token is already set via whitebox in new(), so issuing again should fail
    state.issue_token_expect_err("Token already issued");
}

// ============================================
// 21. Query Agent Token ID
// ============================================

#[test]
fn test_query_agent_token_id() {
    let mut state = AgentTestState::new();
    let token_id = state.query_agent_token_id();
    assert_eq!(token_id, AGENT_TOKEN.to_token_identifier());
}

// ============================================
// 22. Query Agents (BiDi mapper)
// ============================================

#[test]
fn test_query_agents_bidi() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    let agents = state.query_agents();
    let entries: Vec<_> = agents.into_iter().collect();
    assert_eq!(entries.len(), 1);
    let (nonce, addr) = entries[0].clone().into_tuple();
    assert_eq!(nonce, 1u64);
    assert_eq!(addr, AGENT_OWNER.to_managed_address());
}

// ============================================
// 23. Query Agent (getAgent view)
// ============================================

#[test]
fn test_query_get_agent_view() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    let agent = state.query_agent(1);
    assert_eq!(agent.name, ManagedBuffer::<StaticApi>::from(b"TestAgent"));
    assert_eq!(
        agent.public_key,
        ManagedBuffer::<StaticApi>::from(b"pubkey123")
    );
}

// ============================================
// 23b. Query Non-Existent Agent (Agent not found guard)
// ============================================

#[test]
fn test_query_nonexistent_agent() {
    let mut state = AgentTestState::new();
    // No agents registered — nonce 99 does not exist
    state.query_agent_expect_err(99, "Agent not found");
    state.query_agent_owner_expect_err(99, "Agent not found");
}

// ============================================
// 24. Query Agent Metadata Bulk
// ============================================

#[test]
fn test_query_agent_metadata_bulk() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1"), (b"key2", b"val2")],
        vec![],
    );

    let bulk = state.query_agent_metadata_bulk(1);
    let entries: Vec<_> = bulk.into_iter().collect();
    assert_eq!(entries.len(), 2);
}

// ============================================
// 25. Query Agent Service Config Bulk
// ============================================

#[test]
fn test_query_agent_service_bulk() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![
            (1u32, 100u64, b"USDC-abcdef", 0u64),
            (2u32, 200u64, b"USDC-abcdef", 0u64),
        ],
    );

    let bulk = state.query_agent_service_bulk(1);
    let entries: Vec<_> = bulk.into_iter().collect();
    assert_eq!(entries.len(), 2);
}

// ============================================
// 26. Admin Config: setIdentityRegistryAddress (validation)
// ============================================

#[test]
fn test_set_identity_registry_address() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array());
    state.set_identity_registry_address(&OWNER_ADDRESS, new_addr);
}

#[test]
fn test_set_identity_registry_address_not_owner() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array());
    state.set_identity_registry_address_expect_err(
        &CLIENT,
        new_addr,
        "Endpoint can only be called by owner",
    );
}

// ============================================
// 27. Admin Config: setIdentityContractAddress (reputation)
// ============================================

#[test]
fn test_set_reputation_identity_address() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array());
    state.set_reputation_identity_address(&OWNER_ADDRESS, new_addr);
}

#[test]
fn test_set_reputation_identity_address_not_owner() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array());
    state.set_reputation_identity_address_expect_err(
        &CLIENT,
        new_addr,
        "Endpoint can only be called by owner",
    );
}

// ============================================
// 28. Admin Config: setValidationContractAddress (reputation)
// ============================================

#[test]
fn test_set_reputation_validation_address() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(VALIDATION_SC_ADDRESS.eval_to_array());
    state.set_reputation_validation_address(&OWNER_ADDRESS, new_addr);
}

#[test]
fn test_set_reputation_validation_address_not_owner() {
    let mut state = AgentTestState::new();

    let new_addr = ManagedAddress::<StaticApi>::from(VALIDATION_SC_ADDRESS.eval_to_array());
    state.set_reputation_validation_address_expect_err(
        &CLIENT,
        new_addr,
        "Endpoint can only be called by owner",
    );
}

// ============================================
// 29. Submit Proof — Nonexistent Job
// ============================================

#[test]
fn test_submit_proof_nonexistent_job() {
    let mut state = AgentTestState::new();
    state.submit_proof_expect_err(&WORKER, b"nonexistent-job", b"proof-data", "Job not found");
}

// ============================================
// 30. Init Job — Duplicate
// ============================================

#[test]
fn test_init_job_duplicate() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-dup", 1, None);
    state.init_job_expect_err(&CLIENT, b"job-dup", 1, None, "Job already initialized");
}

// ============================================
// 31. Init Job — Wrong Payment Token
// ============================================

#[test]
fn test_init_job_wrong_token() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    state.init_job_with_wrong_token_expect_err(
        &CLIENT,
        b"job-wrong-tok",
        1,
        1,
        "WRONG-abcdef",
        0,
        100,
        "Invalid payment token",
    );
}

// ============================================
// 32. Set Metadata — Not Agent Owner
// ============================================

#[test]
fn test_set_metadata_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.set_metadata_expect_err(
        &CLIENT,
        1,
        vec![(b"key1", b"val1")],
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 33. Set Service Configs — Not Agent Owner
// ============================================

#[test]
fn test_set_service_configs_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.set_service_configs_expect_err(
        &CLIENT,
        1,
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 34. Remove Metadata — Not Agent Owner
// ============================================

#[test]
fn test_remove_metadata_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![(b"key1", b"val1")],
        vec![],
    );

    state.remove_metadata_expect_err(
        &CLIENT,
        1,
        vec![b"key1"],
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 35. Remove Service Configs — Not Agent Owner
// ============================================

#[test]
fn test_remove_service_configs_not_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![(1u32, 100u64, b"USDC-abcdef", 0u64)],
    );

    state.remove_service_configs_expect_err(
        &CLIENT,
        1,
        vec![1],
        "Only the agent owner can perform this action",
    );
}

// ============================================
// 36. Submit Proof — Agent Owner also allowed
// ============================================

#[test]
fn test_submit_proof_agent_owner() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-owner-proof", 1, None);
    // Agent owner can also call submit_proof
    state.submit_proof(&AGENT_OWNER, b"job-owner-proof", b"proof_from_owner");

    let job = state.query_job_data(b"job-owner-proof");
    if let OptionalValue::Some(data) = job {
        assert_eq!(data.status, JobStatus::Pending);
    }
}

// ============================================
// 37. Submit Feedback — Employer can submit without validation
// ============================================

#[test]
fn test_give_feedback_simple_no_validation_needed() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-no-val", 1, None);
    // ERC-8004: employer can submit feedback without needing validation
    state.give_feedback_simple(&CLIENT, b"job-no-val", 1, 80);

    let score = state.query_reputation_score(1);
    assert_eq!(score, BigUint::<StaticApi>::from(80u64));
}

// ============================================
// 38. Submit Feedback — Wrong Caller (not employer)
// ============================================

#[test]
fn test_give_feedback_simple_not_employer() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-wrong-caller", 1, None);
    state.submit_proof(&WORKER, b"job-wrong-caller", b"proof");

    // WORKER (not employer) tries to submit feedback
    state.give_feedback_simple_expect_err(
        &WORKER,
        b"job-wrong-caller",
        1,
        80,
        "Only the employer can provide feedback",
    );
}

// ============================================
// 39. Append Response — Permissionless (ERC-8004)
// ============================================

#[test]
fn test_append_response_permissionless() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job-resp", 1, None);
    state.submit_proof(&WORKER, b"job-resp", b"proof");

    // ERC-8004: anyone can append response — CLIENT can do it
    state.append_response(&CLIENT, b"job-resp", b"https://response.uri");

    let response = state.query_agent_response(b"job-resp");
    assert_eq!(
        response,
        ManagedBuffer::<StaticApi>::from(b"https://response.uri")
    );
}

// ============================================
// 40. Clean Old Jobs — Job Not Old Enough
// ============================================

#[test]
fn test_clean_old_jobs_not_old_enough() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.world.current_block().block_timestamp_millis(0);
    state.init_job(&CLIENT, b"job_recent", 1, None);

    // Advance only 1 day (< 3 days threshold)
    let one_day_ms: u64 = 1 * 24 * 60 * 60 * 1000;
    state
        .world
        .current_block()
        .block_timestamp_millis(one_day_ms);

    state.clean_old_jobs(vec![b"job_recent"]);

    // Job should still exist
    let job = state.query_job_data(b"job_recent");
    assert!(job.is_some(), "Job should not be cleaned — not old enough");
}

// ============================================
// 41. Update Agent — Invalid NFT (wrong nonce)
// ============================================

#[test]
fn test_update_agent_invalid_nft() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    // Try updating with wrong NFT nonce (2 doesn't exist)
    state.update_agent_expect_err(
        &AGENT_OWNER,
        2,
        b"NewName",
        b"https://new.uri",
        b"newpubkey",
        "insufficient funds",
    );
}

// ============================================
// 43a. Update Agent — happy path (basic)
// ============================================

#[test]
#[ignore] // Requires ESDTMetaDataRecreate VM mock (not in official SDK yet)
fn test_update_agent() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    // Update name, uri, and public key
    state.update_agent_raw(
        &AGENT_OWNER,
        1,
        b"UpdatedAgent",
        b"https://updated.example.com",
        b"newpubkey",
        None,
        None,
    );

    // Agent owner preserved after update
    let owner = state.query_agent_owner(1);
    assert_eq!(owner, ManagedAddress::from(AGENT_OWNER.to_address()),);
}

// ============================================
// 43b. Update Agent — with metadata and services
// ============================================

#[test]
#[ignore] // Requires ESDTMetaDataRecreate VM mock (not in official SDK yet)
fn test_update_agent_with_meta_and_services() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.update_agent_raw(
        &AGENT_OWNER,
        1,
        b"UpdatedAgent",
        b"https://updated.example.com",
        b"newpubkey",
        Some(vec![(b"bio", b"Updated bio")]),
        Some(vec![(1, 100, b"EGLD-000000", 0)]),
    );

    // Verify metadata was updated
    let meta = state.query_metadata(1, b"bio");
    assert!(meta.is_some());

    // Verify service config was set
    let svc = state.query_service_config(1, 1);
    assert!(svc.is_some());
}

// ============================================
// 44. Upgrade Identity Registry
// ============================================

#[test]
fn test_upgrade_identity() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.upgrade_identity();

    let details = state.query_agent_details(1);
    assert_eq!(details.name, ManagedBuffer::<StaticApi>::from(b"TestAgent"));
}

// ============================================
// 45. Upgrade Validation Registry
// ============================================

#[test]
fn test_upgrade_validation() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &AGENT_OWNER,
        b"TestAgent",
        b"https://agent.example.com",
        b"pubkey123",
        vec![],
        vec![],
    );

    state.init_job(&CLIENT, b"job_upgrade", 1, None);
    state.upgrade_validation();

    let job = state.query_job_data(b"job_upgrade");
    assert!(job.is_some(), "Job should persist after upgrade");
}

// ============================================
// 46. Upgrade Reputation Registry
// ============================================

#[test]
fn test_upgrade_reputation() {
    let mut state = AgentTestState::new();
    state.upgrade_reputation();

    // After upgrade, config should still be intact
    let validation_addr = state.query_validation_contract_address();
    assert_eq!(
        validation_addr,
        ManagedAddress::<StaticApi>::from(VALIDATION_SC_ADDRESS.eval_to_array())
    );
}

// ============================================
// 47. Query Reputation Contract Addresses
// ============================================

#[test]
fn test_query_reputation_contract_addresses() {
    let mut state = AgentTestState::new();

    let validation_addr = state.query_validation_contract_address();
    assert_eq!(
        validation_addr,
        ManagedAddress::<StaticApi>::from(VALIDATION_SC_ADDRESS.eval_to_array())
    );

    let identity_addr = state.query_identity_contract_address();
    assert_eq!(
        identity_addr,
        ManagedAddress::<StaticApi>::from(IDENTITY_SC_ADDRESS.eval_to_array())
    );
}

// ============================================
// 48. is_job_verified — View after validation
// ============================================

#[test]
fn test_is_job_verified_view() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &OWNER_ADDRESS,
        b"IsVerifiedBot",
        b"https://example.com/manifest",
        &[0u8; 32],
        vec![(b"type", b"bot")],
        vec![],
    );
    state.init_job(&OWNER_ADDRESS, b"job_verify_view", 1, None);

    // Not verified before validation
    assert!(!state.query_is_job_verified(b"job_verify_view"));

    // Submit proof + validation
    state.submit_proof(&OWNER_ADDRESS, b"job_verify_view", b"proof-hash");
    state.validation_request(
        &OWNER_ADDRESS,
        b"job_verify_view",
        &VALIDATOR,
        b"https://oracle.example.com/verify",
        b"req-verify-view",
    );
    state.validation_response(
        &VALIDATOR,
        b"req-verify-view",
        1,
        b"https://oracle.example.com/result",
        b"resp-verify-view",
        b"approved",
    );

    // Now verified
    assert!(state.query_is_job_verified(b"job_verify_view"));
}

// ============================================
// 49. Multi-Job Reputation Weighted Average
// ============================================

#[test]
fn test_multi_job_reputation_average() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &OWNER_ADDRESS,
        b"MultiRepBot",
        b"https://example.com/manifest",
        &[0u8; 32],
        vec![(b"type", b"worker")],
        vec![],
    );

    // Job 1: rating 80
    state.init_job(&OWNER_ADDRESS, b"rep_avg_1", 1, None);
    state.submit_proof(&OWNER_ADDRESS, b"rep_avg_1", b"proof-1");
    state.validation_request(
        &OWNER_ADDRESS,
        b"rep_avg_1",
        &VALIDATOR,
        b"https://oracle.example.com/verify",
        b"req-avg-1",
    );
    state.validation_response(
        &VALIDATOR,
        b"req-avg-1",
        1,
        b"https://oracle.example.com/result",
        b"resp-avg-1",
        b"approved",
    );
    state.give_feedback_simple(&OWNER_ADDRESS, b"rep_avg_1", 1, 80);

    // Job 2: rating 60
    state.init_job(&OWNER_ADDRESS, b"rep_avg_2", 1, None);
    state.submit_proof(&OWNER_ADDRESS, b"rep_avg_2", b"proof-2");
    state.validation_request(
        &OWNER_ADDRESS,
        b"rep_avg_2",
        &VALIDATOR,
        b"https://oracle.example.com/verify",
        b"req-avg-2",
    );
    state.validation_response(
        &VALIDATOR,
        b"req-avg-2",
        1,
        b"https://oracle.example.com/result",
        b"resp-avg-2",
        b"approved",
    );
    state.give_feedback_simple(&OWNER_ADDRESS, b"rep_avg_2", 1, 60);

    // Job 3: rating 100
    state.init_job(&OWNER_ADDRESS, b"rep_avg_3", 1, None);
    state.submit_proof(&OWNER_ADDRESS, b"rep_avg_3", b"proof-3");
    state.validation_request(
        &OWNER_ADDRESS,
        b"rep_avg_3",
        &VALIDATOR,
        b"https://oracle.example.com/verify",
        b"req-avg-3",
    );
    state.validation_response(
        &VALIDATOR,
        b"req-avg-3",
        1,
        b"https://oracle.example.com/result",
        b"resp-avg-3",
        b"approved",
    );
    state.give_feedback_simple(&OWNER_ADDRESS, b"rep_avg_3", 1, 100);

    // Average should be (80+60+100)/3 = 80
    let score = state.query_reputation_score(1);
    assert_eq!(score, 80, "Weighted average of 3 jobs (80+60+100)/3 = 80");

    let total = state.query_total_jobs(1);
    assert_eq!(total, 3, "Should have 3 total jobs");
}

// ============================================
// 50. submit_proof_with_nft — Happy Path
// ============================================

// NOTE: submit_proof_with_nft uses storage_mapper_from_address to cross-read the
// identity-registry's agentTokenId. The RustVM test environment fully supports this
// because both contracts share the same test world.

#[test]
fn test_submit_proof_with_nft_happy_path() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &OWNER_ADDRESS,
        b"NFTProofBot",
        b"https://example.com/manifest",
        &[0u8; 32],
        vec![(b"type", b"worker")],
        vec![],
    );
    state.init_job(&OWNER_ADDRESS, b"job_nft_proof", 1, None);

    // Owner holds AGENT-abcdef nonce=1 after register_agent
    state.submit_proof_with_nft(
        &OWNER_ADDRESS,
        b"job_nft_proof",
        b"nft-proof-hash",
        &AGENT_TOKEN,
        1,
    );

    // Verify proof was stored — job should be Pending after proof
    let job_data = state.query_job_data(b"job_nft_proof");
    match job_data {
        OptionalValue::Some(job) => {
            assert_eq!(
                job.proof,
                ManagedBuffer::<StaticApi>::from(b"nft-proof-hash")
            );
        }
        OptionalValue::None => panic!("Job data should exist after submit_proof_with_nft"),
    }
}

// ============================================
// 51. submit_proof_with_nft — Nonexistent Job
// ============================================

// NOTE: Wrong-token and wrong-nonce error paths are tested via chain-sim (Suite Q/S)
// because the RustVM validates ESDT balances before contract execution,
// making it impossible to send tokens the caller doesn't hold.

#[test]
fn test_submit_proof_with_nft_nonexistent_job() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &OWNER_ADDRESS,
        b"NFTErrBot",
        b"https://example.com/manifest",
        &[0u8; 32],
        vec![(b"type", b"worker")],
        vec![],
    );

    // Try to submit proof for a job that doesn't exist
    state.submit_proof_with_nft_expect_err(
        &OWNER_ADDRESS,
        b"nonexistent_job",
        b"nft-proof-hash",
        &AGENT_TOKEN,
        1,
        "Job not found",
    );
}

// ============================================
// 52. Progressive validation_response (multiple calls)
// ============================================

#[test]
fn test_validation_response_progressive() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &OWNER_ADDRESS,
        b"ProgressiveBot",
        b"https://example.com/manifest",
        &[0u8; 32],
        vec![(b"type", b"validator")],
        vec![],
    );
    state.init_job(&OWNER_ADDRESS, b"job_progressive", 1, None);
    state.submit_proof(&OWNER_ADDRESS, b"job_progressive", b"initial-proof");
    state.validation_request(
        &OWNER_ADDRESS,
        b"job_progressive",
        &VALIDATOR,
        b"https://oracle.example.com/verify",
        b"req-progressive",
    );

    // First validation_response — partial score
    state.validation_response(
        &VALIDATOR,
        b"req-progressive",
        50,
        b"https://oracle.example.com/partial",
        b"resp-partial",
        b"partial",
    );
    assert!(state.query_is_job_verified(b"job_progressive"));

    // Second (progressive) validation_response — updated score
    state.validation_response(
        &VALIDATOR,
        b"req-progressive",
        95,
        b"https://oracle.example.com/final",
        b"resp-final",
        b"approved",
    );
    // Job should still be verified
    assert!(state.query_is_job_verified(b"job_progressive"));
}

// ============================================
// 53. submit_feedback with rating boundary values (0, 100)
// ============================================

#[test]
fn test_give_feedback_simple_rating_boundaries() {
    let mut state = AgentTestState::new();
    state.register_agent(
        &OWNER_ADDRESS,
        b"BoundaryBot",
        b"https://example.com/manifest",
        &[0u8; 32],
        vec![(b"type", b"worker")],
        vec![],
    );

    // Job 1: rating = 0 (lowest possible)
    state.init_job(&OWNER_ADDRESS, b"job_boundary_0", 1, None);
    state.submit_proof(&OWNER_ADDRESS, b"job_boundary_0", b"proof-zero");
    state.validation_request(
        &OWNER_ADDRESS,
        b"job_boundary_0",
        &VALIDATOR,
        b"https://oracle.example.com/verify",
        b"req-boundary-0",
    );
    state.validation_response(
        &VALIDATOR,
        b"req-boundary-0",
        1,
        b"https://oracle.example.com/result",
        b"resp-boundary-0",
        b"approved",
    );
    state.give_feedback_simple(&OWNER_ADDRESS, b"job_boundary_0", 1, 0);

    let score_after_zero = state.query_reputation_score(1);
    assert_eq!(score_after_zero, 0, "Rating 0 → score should be 0");

    // Job 2: rating = 100 → avg should be (0 + 100) / 2 = 50
    state.init_job(&OWNER_ADDRESS, b"job_boundary_100", 1, None);
    state.submit_proof(&OWNER_ADDRESS, b"job_boundary_100", b"proof-hundred");
    state.validation_request(
        &OWNER_ADDRESS,
        b"job_boundary_100",
        &VALIDATOR,
        b"https://oracle.example.com/verify",
        b"req-boundary-100",
    );
    state.validation_response(
        &VALIDATOR,
        b"req-boundary-100",
        1,
        b"https://oracle.example.com/result",
        b"resp-boundary-100",
        b"approved",
    );
    state.give_feedback_simple(&OWNER_ADDRESS, b"job_boundary_100", 1, 100);

    let score_after_hundred = state.query_reputation_score(1);
    assert_eq!(score_after_hundred, 50, "Average of (0 + 100) / 2 = 50");
}

// ============================================
// 54. register_agent on contract with no token issued
// ============================================

#[test]
fn test_register_agent_token_not_issued() {
    let mut state = AgentTestState::new_no_token();
    state.register_agent_expect_err(
        &AGENT_OWNER,
        b"ShouldFail",
        b"https://example.com/manifest",
        b"pubkey123",
        "Token not issued",
    );
}
