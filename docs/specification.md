# MX-8004: Trustless Agents Standard Specification

## 1. Identity Registry

The Identity Registry handles the lifecycle of Agent Identities via Soulbound NFTs.

### 1.1 Agent Registration
`register_agent(name, uri, public_key, metadata)`
- Mints a unique NFT for the agent.
- Stores `AgentDetails` in NFT attributes.
- **NEW**: Synchronizes metadata prices (keys starting with `price:`), tokens (`token:`), and nonces (`pnonce:`) into contract storage for high-performance direct reading.
- Defaults service costs to 0 and payment token to EGLD if not explicitly set.

### 1.2 Metadata Updates
`update_agent(new_uri, new_public_key, metadata)`
- **Transfer-Execute Pattern**: The agent owner (or bot) sends its Soulbound NFT to this endpoint.
- **Nonce Discovery**: The contract automatically extracts the `nonce` from the NFT payment.
- **Validation**: Ensures the sent token matches the `agent_token_id`.
- **Atomic Update**: Contract updates attributes, synchronizes pricing storage, and returns the NFT to the sender in the same transaction.
- Facilitates secure, bot-driven metadata management.

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

---

> [!IMPORTANT]
> **TODO: Verification Phase**
> Once the contract is updated, all dependent services (Facilitator, Relayer, MCP Server) MUST be verified to ensure they correctly handle zero-cost services and use the updated registration/update endpoints.
