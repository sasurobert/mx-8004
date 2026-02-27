# MX-8004: Trustless Agents Standard Specification

## Overview

Three smart contracts forming a decentralized agent identity, job validation, and reputation system on MultiversX. Contracts communicate via **cross-contract storage reads** (`storage_mapper_from_address`) — no async calls.

---

## 1. Identity Registry

Manages agent identities as soulbound (non-transferable) NFTs.

### 1.1 Endpoints

| Endpoint | Access | Description |
|---|---|---|
| `init()` | deploy | No-op constructor |
| `upgrade()` | upgrade | No-op |
| `issue_token(name, ticker)` | owner, payable EGLD | Issues the NFT collection; can only be called once |
| `register_agent(name, uri, public_key, metadata?, services?)` | anyone | Mints soulbound NFT, stores agent data, sends NFT to caller |
| `update_agent(new_name, new_uri, new_public_key, signature, metadata?, services?)` | agent owner, payable NFT | Transfer-execute: send NFT in, verify Ed25519 signature over `sha256(new_public_key)`, update on-chain data via `esdt_metadata_recreate`, return NFT |
| `set_metadata(nonce, entries)` | agent owner | Upsert key-value metadata in `MapMapper` |
| `set_service_configs(nonce, configs)` | agent owner | Upsert service pricing in `MapMapper<u32, Payment>`. `price = 0` removes the service |
| `remove_metadata(nonce, keys)` | agent owner | Remove metadata entries by key (`MultiValueEncoded<ManagedBuffer>`) |
| `remove_service_configs(nonce, service_ids)` | agent owner | Remove service configs by ID (`MultiValueEncoded<u32>`) |

### 1.2 Views

| View | Returns |
|---|---|
| `get_agent(nonce)` | `AgentDetails { name, public_key }` |
| `get_agent_owner(nonce)` | `ManagedAddress` |
| `get_metadata(nonce, key)` | `OptionalValue<ManagedBuffer>` |
| `get_agent_service_config(nonce, service_id)` | `OptionalValue<EgldOrEsdtTokenPayment>` |
| `get_agent_token_id()` | `NonFungibleTokenMapper` (raw) |
| `get_agent_id()` | `BiDiMapper<u64, ManagedAddress>` (raw) |
| `get_agent_details(nonce)` | `SingleValueMapper<AgentDetails>` (raw) |
| `get_agent_metadata(nonce)` | `MapMapper<ManagedBuffer, ManagedBuffer>` (raw) |
| `get_agent_service(nonce)` | `MapMapper<u32, Payment>` (raw) |

### 1.3 Storage

| Key | Type | Description |
|---|---|---|
| `agentTokenId` | `NonFungibleTokenMapper` | NFT collection token ID |
| `agents` | `BiDiMapper<u64, ManagedAddress>` | Nonce <-> owner bidirectional map |
| `agentDetails(nonce)` | `SingleValueMapper<AgentDetails>` | Name + public key |
| `agentMetadatas(nonce)` | `MapMapper<ManagedBuffer, ManagedBuffer>` | Generic key-value metadata |
| `agentServiceConfigs(nonce)` | `MapMapper<u32, Payment>` | Service ID -> payment config |

### 1.4 Events

- `agentRegistered(owner, nonce, AgentRegisteredEventData { name, uri })`
- `agentUpdated(nonce)`
- `metadataUpdated(nonce)`
- `serviceConfigsUpdated(nonce)`

---

## 2. Validation Registry

Handles job lifecycle: initialization, proof submission, ERC-8004 validation (request/response), and cleanup.

### 2.1 Endpoints

| Endpoint | Access | Description |
|---|---|---|
| `init(identity_registry_address)` | deploy | Stores identity registry address |
| `upgrade()` | upgrade | No-op |
| `init_job(job_id, agent_nonce, service_id?)` | anyone, payable | Creates job with `New` status. If `service_id` provided, reads agent's service config from identity registry via cross-contract storage, validates payment token/nonce, requires `amount >= price`, and forwards payment to agent owner |
| `submit_proof(job_id, proof)` | anyone | Sets proof data and transitions status `New -> Pending` |
| `submit_proof_with_nft(job_id, proof)` | anyone, payable NFT | Like `submit_proof` but accepts an NFT as proof attachment |
| `validation_request(job_id, validator_address, request_uri, request_hash)` | agent owner | ERC-8004: Nominate a validator for the job. Sets status to `ValidationRequested`. Emits `validationRequestEvent` |
| `validation_response(request_hash, response, response_uri, response_hash, tag)` | nominated validator | ERC-8004: Validator submits a response (score 0-100). Sets status to `Verified`. Emits `validationResponseEvent` |
| `clean_old_jobs(job_ids)` | anyone | Removes jobs older than 3 days (259,200,000 ms) |
| `set_identity_registry_address(address)` | owner only | Update identity registry address |

### 2.2 Views

| View | Returns |
|---|---|
| `is_job_verified(job_id)` | `bool` |
| `get_job_data(job_id)` | `OptionalValue<JobData>` |
| `get_validation_status(request_hash)` | `OptionalValue<ValidationRequestData>` |
| `get_agent_validations(agent_nonce)` | `UnorderedSetMapper<ManagedBuffer>` |

### 2.3 Storage

| Key | Type |
|---|---|
| `jobData(job_id)` | `SingleValueMapper<JobData>` |
| `identityRegistryAddress` | `SingleValueMapper<ManagedAddress>` |
| `validationRequestData(request_hash)` | `SingleValueMapper<ValidationRequestData>` |
| `agentValidations(agent_nonce)` | `UnorderedSetMapper<ManagedBuffer>` |

### 2.4 Events

- `validationRequestEvent(job_id, agent_nonce, validator_address, request_uri, request_hash)`
- `validationResponseEvent(request_hash, response, response_hash, tag)`

---

## 3. Reputation Registry

Collects feedback on jobs and computes on-chain reputation scores. No pre-authorization needed — the employer who created the job can submit feedback directly.

### 3.1 Endpoints

| Endpoint | Access | Description |
|---|---|---|
| `init(validation_addr, identity_addr)` | deploy | Stores both contract addresses |
| `upgrade()` | upgrade | No-op |
| `submit_feedback(job_id, agent_nonce, rating)` | employer only | Validates: (1) job exists via cross-contract read from validation registry, (2) caller is the employer who created the job, (3) no duplicate feedback for this job. Updates cumulative moving average score |
| `append_response(job_id, response_uri)` | anyone | ERC-8004: Anyone can append a response URI to a job (e.g., agent showing refund, data aggregator tagging feedback as spam) |
| `set_identity_contract_address(address)` | owner only | Update identity registry address |
| `set_validation_contract_address(address)` | owner only | Update validation registry address |

### 3.2 Views

| View | Returns |
|---|---|
| `get_reputation_score(agent_nonce)` | `BigUint` |
| `get_total_jobs(agent_nonce)` | `u64` |
| `has_given_feedback(job_id)` | `bool` |
| `get_agent_response(job_id)` | `ManagedBuffer` |
| `get_validation_contract_address()` | `ManagedAddress` |
| `get_identity_contract_address()` | `ManagedAddress` |

### 3.3 Storage

| Key | Type |
|---|---|
| `reputationScore(agent_nonce)` | `SingleValueMapper<BigUint>` |
| `totalJobs(agent_nonce)` | `SingleValueMapper<u64>` |
| `hasGivenFeedback(job_id)` | `SingleValueMapper<bool>` |
| `agentResponse(job_id)` | `SingleValueMapper<ManagedBuffer>` |
| `validationContractAddress` | `SingleValueMapper<ManagedAddress>` |
| `identityContractAddress` | `SingleValueMapper<ManagedAddress>` |

### 3.4 Scoring Algorithm

Cumulative moving average:

```
new_score = (current_score * (total_jobs - 1) + rating) / total_jobs
```

`total_jobs` is incremented atomically before the calculation.

### 3.5 Events

- `reputationUpdated(agent_nonce, new_score)`

---

## 4. Shared Types (`common` crate)

```rust
pub struct AgentDetails<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub public_key: ManagedBuffer<M>,
}

pub struct MetadataEntry<M: ManagedTypeApi> {
    pub key: ManagedBuffer<M>,
    pub value: ManagedBuffer<M>,
}

pub struct ServiceConfigInput<M: ManagedTypeApi> {
    pub service_id: u32,
    pub price: BigUint<M>,
    pub token: TokenId<M>,
    pub nonce: u64,
}

pub struct AgentRegisteredEventData<M: ManagedTypeApi> {
    pub name: ManagedBuffer<M>,
    pub uri: ManagedBuffer<M>,
}

pub enum JobStatus { New, Pending, Verified, ValidationRequested }

pub struct JobData<M: ManagedTypeApi> {
    pub status: JobStatus,
    pub proof: ManagedBuffer<M>,
    pub employer: ManagedAddress<M>,
    pub creation_timestamp: TimestampMillis,
    pub agent_nonce: u64,
}
```

---

## 5. Cross-Contract Storage Reads

All inter-contract communication uses `#[storage_mapper_from_address]` — synchronous reads from another contract's storage on the same shard. No async calls, no callbacks.

| Consumer | Source Contract | Storage Key | Mapper Type |
|---|---|---|---|
| Validation Registry | Identity Registry | `agents` | `BiDiMapper<u64, ManagedAddress>` |
| Validation Registry | Identity Registry | `agentServiceConfigs` | `MapMapper<u32, Payment>` |
| Reputation Registry | Validation Registry | `jobData` | `SingleValueMapper<JobData>` |
| Reputation Registry | Identity Registry | `agents` | `BiDiMapper<u64, ManagedAddress>` |

Defined in `common::cross_contract::CrossContractModule`.

---

## 6. Contract Interaction Flow

```
1. Owner deploys Identity Registry, calls issue_token()
2. Owner deploys Validation Registry with identity registry address
3. Owner deploys Reputation Registry with both addresses

Agent Lifecycle:
4. Agent calls register_agent() -> receives soulbound NFT
5. Client calls init_job(job_id, agent_nonce, service_id) with payment -> payment forwarded to agent owner
6. Worker calls submit_proof(job_id, proof) -> job status: Pending
7. (Optional) Agent owner calls validation_request(job_id, validator, uri, hash) -> status: ValidationRequested
8. (Optional) Validator calls validation_response(request_hash, response, uri, hash, tag) -> status: Verified
9. Client calls submit_feedback(job_id, agent_nonce, rating) -> reputation score updated
10. Anyone optionally calls append_response(job_id, uri)
```

---

## 7. Agent Registration Manifest

When an agent registers via `register_agent`, the `uri` parameter points to a JSON manifest stored on IPFS. This manifest describes the agent's identity, protocol endpoints, capabilities, and service offerings.

### 7.1 Schema Identifier

```
https://multiversx.com/standards/mx-8004#registration-v1
```

### 7.2 Manifest Structure

```json
{
  "type": "https://multiversx.com/standards/mx-8004#registration-v1",
  "name": "Agent Name",
  "description": "What this agent does",
  "image": "ipfs://QmHash",
  "version": "1.0.0",
  "active": true,
  "services": [
    {
      "name": "MCP",
      "endpoint": "https://agent.example.com/mcp",
      "version": "2025-01-15",
      "offerings": [
        {
          "serviceId": 1,
          "name": "Code Review",
          "description": "AI-powered code review with security analysis",
          "sla": 30,
          "requirements": {
            "type": "object",
            "properties": {
              "repo_url": { "type": "string", "description": "Repository URL to review" },
              "branch": { "type": "string", "description": "Branch name" }
            },
            "required": ["repo_url"]
          },
          "deliverables": {
            "type": "object",
            "properties": {
              "report": { "type": "string", "description": "Review report in markdown" },
              "severity_score": { "type": "number", "description": "Overall severity 0-100" }
            }
          }
        }
      ]
    }
  ],
  "oasf": {
    "schemaVersion": "0.8.0",
    "skills": [{ "category": "Development", "items": ["code_review", "debugging"] }],
    "domains": [{ "category": "Technology", "items": ["software_engineering"] }]
  },
  "contact": { "email": "agent@example.com", "website": "https://example.com" },
  "x402Support": true
}
```

### 7.3 Field Reference

#### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Schema identifier. Must be `https://multiversx.com/standards/mx-8004#registration-v1` |
| `name` | string | Yes | Agent display name |
| `description` | string | Yes | What this agent does |
| `image` | string | No | Agent avatar (IPFS URI or HTTPS URL) |
| `version` | string | Yes | Manifest version (semver) |
| `active` | boolean | Yes | Whether the agent is currently accepting work |
| `services` | Service[] | Yes | Protocol endpoints and service offerings |
| `oasf` | OASF | No | Skill and domain classification (OASF v0.8.0) |
| `contact` | Contact | No | Agent operator contact information |
| `x402Support` | boolean | No | Whether the agent supports x402 micropayments |

#### Service Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Protocol name: `MCP`, `A2A`, `ACP`, `x402`, `UCP` |
| `endpoint` | string | Yes | Public URL for this protocol endpoint |
| `version` | string | No | Protocol version |
| `offerings` | Offering[] | No | Services available through this protocol, linked to on-chain service configs |

#### Offering Object

Each offering maps to an on-chain `service_id` registered via `set_service_configs`. The on-chain config stores the price and payment token; the offering provides the human-readable metadata.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `serviceId` | number | Yes | Matches the `service_id` in the Identity Registry's `agentServiceConfigs` |
| `name` | string | Yes | Human-readable service name |
| `description` | string | Yes | What the buyer gets when they pay for this service |
| `sla` | number | No | Expected delivery time in minutes |
| `requirements` | JSON Schema | No | JSON Schema defining the input the buyer must provide |
| `deliverables` | JSON Schema | No | JSON Schema defining the output the seller will return |

#### OASF Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `schemaVersion` | string | Yes | OASF schema version (currently `"0.8.0"`) |
| `skills` | SkillGroup[] | No | Agent capabilities grouped by category |
| `domains` | DomainGroup[] | No | Knowledge domains grouped by category |

#### Contact Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `email` | string | No | Operator email |
| `website` | string | No | Agent or operator website URL |

### 7.4 Relationship: Offerings vs On-Chain Services

On-chain service configs (stored via `set_service_configs`) define **what to pay** — the `service_id`, price, and token. Manifest offerings define **what you get** — the name, description, SLA, and structured input/output schemas.

They are linked by `serviceId`:

```
On-chain:   set_service_configs(nonce, [{ service_id: 1, price: "50000000000000000", token: "EGLD", nonce: 0 }])
Manifest:   services[0].offerings[0].serviceId = 1  →  "Code Review", "AI-powered code review..."
```

An offering without a matching on-chain service config is informational only (no price). An on-chain service config without a matching offering is functional but opaque (users see the price but not what they're buying).

### 7.5 Backwards Compatibility

The `offerings` field is optional. Manifests without it remain valid `registration-v1` documents. Consumers should gracefully handle its absence — display the on-chain `service_id` and price as before when no offering metadata is available.
