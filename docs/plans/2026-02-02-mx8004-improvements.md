# [Feature Name] Implementation Plan: MX-8004 Security & Quality Improvements

**Goal:** Implement storage cleanup, authorization gates, and scenario testing.

**Architecture:** Workspace-wide cleanup, employer-linking, and scenario verification.

**Tech Stack:** multiversx-sc, Rust, Scenarios.

---

### Task 1: Fix Existing Linter & WASM Issues

**Files:**
- Modify: `mx-8004/validation-registry/tests/validation_registry_test.rs`
- Modify: `mx-8004/reputation-registry/tests/reputation_registry_test.rs`
- Modify: `mx-8004/identity-registry/src/lib.rs`
- Modify: `mx-8004/validation-registry/wasm/src/lib.rs`
- Modify: `mx-8004/reputation-registry/wasm/src/lib.rs`

**Step 1: Fix clippy warnings**
- Replace `assert_eq!(..., true)` with `assert!(...)`.
- Use `let _ =` for `execute_query`.
- Remove needless borrows.

**Step 2: Fix WASM boilerplates**
- Ensure `endpoints!` macro is correctly defined or empty.

---

### Task 2: Validation Registry Enhancements (Employer-Linked Jobs)

**Files:**
- Modify: `mx-8004/validation-registry/src/lib.rs`

**Step 1: Add employer and timestamp storage**
```rust
#[storage_mapper("jobEmployer")]
fn job_employer(&self, job_id: ManagedBuffer) -> SingleValueMapper<ManagedAddress>;

#[storage_mapper("jobCreationTimestamp")]
fn job_creation_timestamp(&self, job_id: ManagedBuffer) -> SingleValueMapper<u64>;
```

**Step 2: Implement init_job**
```rust
#[endpoint(init_job)]
fn init_job(&self, job_id: ManagedBuffer) {
    require!(self.job_employer(job_id.clone()).is_empty(), "Job already initialized");
    self.job_employer(job_id.clone()).set(self.blockchain().get_caller());
    self.job_creation_timestamp(job_id).set(self.blockchain().get_block_timestamp());
}
```

**Step 3: Implement gas-aware cleanup**
```rust
#[endpoint(clean_old_jobs)]
fn clean_old_jobs(&self, job_ids: MultiValueEncoded<ManagedBuffer>) {
    // Cleanup logic
}
```

---

### Task 3: Reputation Registry (Authorization Gate & Responses)

**Files:**
- Modify: `mx-8004/reputation-registry/src/lib.rs`

**Step 1: Implement Authorization Gate**
```rust
#[storage_mapper("isFeedbackAuthorized")]
fn is_feedback_authorized(&self, job_id: ManagedBuffer, client: ManagedAddress) -> SingleValueMapper<bool>;

#[endpoint(authorize_feedback)]
fn authorize_feedback(&self, job_id: ManagedBuffer, client: ManagedAddress) {
    // Only the agent linked to the job can call this
    self.is_feedback_authorized(job_id, client).set(true);
}
```

**Step 2: Frontrunning Protection**
- Compare `caller` with `ValidationRegistry::get_job_employer(job_id)`.

**Step 3: Implement Agent Response**
```rust
#[endpoint(append_response)]
fn append_response(&self, job_id: ManagedBuffer, response_uri: ManagedBuffer) {
    // Store response linked to feedback
}
```

---

### Task 4: Scenario-Based Integration Tests

**Files:**
- Create: `mx-8004/scenarios/authenticity_loop.scen.json`
- Create: `mx-8004/tests/scenario_rs_test.rs`

**Step 1: Write the full flow scenario**
[JSON defining the full loop: ID -> VAL -> INIT -> AUTH -> FEEDBACK]

**Step 2: Create the Rust runner**
```rust
#[test]
fn authenticity_loop_scen() {
    multiversx_sc_scenario::run_rs("scenarios/authenticity_loop.scen.json", world());
}
```
---
