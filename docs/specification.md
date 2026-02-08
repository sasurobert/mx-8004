# MX-8004: Trustless Agents Standard Specification

## 1. Identity Registry

The Identity Registry handles the lifecycle of Agent Identities via Soulbound NFTs.

### 1.1 Agent Registration
`register_agent(name, uri, public_key, metadata, service_configs)`
- Mints a unique NFT for the agent.
- Stores `AgentDetails` in NFT attributes.
- `metadata` — generic key-value pairs stored in NFT attributes for extensibility.
- `service_configs` — typed `ServiceConfigInput` structs written directly to storage mappers:
  - `ServiceConfigInput { service_id, price, token, pnonce }`
- If `service_configs` is empty, defaults to service `"0"` with price 0, EGLD, pnonce 0.

### 1.2 Agent Updates
`update_agent(new_uri, new_public_key, metadata, service_configs)`
- **Transfer-Execute Pattern**: The agent owner sends its Soulbound NFT to this endpoint.
- **Nonce Discovery**: The contract automatically extracts the `nonce` from the NFT payment.
- **Validation**: Ensures the sent token matches the `agent_token_id`.
- **Atomic Update**: Contract updates attributes, writes service configs to storage, and returns the NFT to the sender in the same transaction.

### 1.3 Metadata Management
`set_metadata(nonce, entries)` — upserts generic key-value metadata in NFT attributes. Does **not** affect pricing storage.

### 1.4 Service Config Management
`set_service_configs(nonce, configs)` — writes `ServiceConfigInput` entries directly to storage mappers. Requires caller to be the agent owner.

## 2. Validation Registry

Handles job initialization and proof verification.

### 2.1 Job Initialization with Payment
`init_job_with_payment(job_id, agent_nonce, service_id)`
- Reads `agent_owner`, `required_price`, `required_token`, and `required_pnonce` directly from `IdentityRegistry` storage.
- If `required_price` is 0, it accepts any payment (including 0).
- Validates that the provided payment matches the `required_token` and `required_pnonce`.
- Forwards payment to `agent_owner`.

## 3. Storage Structures

### Identity Registry
- `agentTokenId`: NonFungibleTokenMapper
- `agentServicePrice(nonce, service_id)`: SingleValueMapper<BigUint>
- `agentServicePaymentToken(nonce, service_id)`: SingleValueMapper<EgldOrEsdtTokenIdentifier>
- `agentServicePaymentNonce(nonce, service_id)`: SingleValueMapper<u64>

### Types
```rust
struct ServiceConfigInput {
    service_id: ManagedBuffer,
    price: BigUint,
    token: EgldOrEsdtTokenIdentifier,
    pnonce: u64,
}
```

---

> [!IMPORTANT]
> **TODO: Verification Phase**
> Once the contract is updated, all dependent services (Facilitator, Relayer, MCP Server) MUST be verified to ensure they correctly handle zero-cost services and use the updated registration/update endpoints.
