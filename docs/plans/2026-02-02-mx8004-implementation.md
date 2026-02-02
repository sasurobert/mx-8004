# MX-8004 Implementation Plan

**Goal:** Implement the MX-8004 Agent Standard on MultiversX, comprising Identity, Validation, and Reputation registries, with a focus on security and authenticity.

**Architecture:** A workspace with three smart contracts (Identity, Validation, Reputation) interacting via synchronous and asynchronous calls. Tested with RustVM and Rust SDK scenarios.

**Tech Stack:** Rust, multiversx-sc, mxpy.

---

### Task 1: Infrastructure & Identity Registry

**Files:**
- Create: `mx-8004/Cargo.toml`
- Create: `mx-8004/mxpy.json`
- Create: `mx-8004/identity-registry/Cargo.toml`
- Create: `mx-8004/identity-registry/src/lib.rs`
- Create: `mx-8004/identity-registry/wasm/Cargo.toml`

**Step 1: Workspace Setup**
Initialize the Rust workspace in `mx-8004` and the `identity-registry` crate.

**Step 2: Identity Storage & Init**
- Define `agent_nft_token_id`, `agent_uri`, `agent_public_key` storage mappers.
- Implement `init` to optionally issue or set the Token Identifier.

**Step 3: Register Agent Endpoint**
- Implement `register_agent(name, uri, pub_key)`.
- Mint an SFT/NFT for the caller.
- Emit `agentRegistered(owner, nonce, name, uri)`.

**Step 4: Update Agent Endpoint**
- Implement `update_agent(nonce, new_uri, new_pub_key)`.
- Verify caller owns the Agent NFT.
- Emit `agentUpdated(nonce, new_uri)`.

**Step 5: Get Agent View**
- Implement `getAgent(nonce)` returning a struct with storage data.

### Task 2: Validation Registry

**Step 1: Validation Storage**
- Define `job_proof(job_id)`, `job_status`.
- Define `authorized_validators`.

**Step 2: Submit Proof**
- `submit_proof(job_id, hash)`. Stores the hash.
- Caller must be the assigned Agent (check ownership/assignment).

**Step 3: Verify Job**
- `verify_job(job_id)`.
- Only callable by `authorized_validators`.
- Sets status to `Verified`.
- Emit `jobVerified(job_id, agent_nonce, status)`.

### Task 3: Reputation Registry & Integration

**Step 1: Reputation Storage**
- `reputationScore(agent)`, `totalJobs`.
- `validation_contract_address`.

**Step 2: Submit Feedback**
- `submit_feedback(job_id, agent_nonce, rating)`.
- **CRITICAL**: Call `validation_contract.is_job_verified(job_id)`.
- Update score if verified.
- Emit `reputationUpdated(agent_nonce, new_score)`.
