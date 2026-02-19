use multiversx_sc::types::{EsdtLocalRole, TestAddress, TestSCAddress};
use multiversx_sc_scenario::imports::{MxscPath, TestTokenIdentifier};

// ── Addresses ──
pub const OWNER_ADDRESS: TestAddress = TestAddress::new("owner");
pub const AGENT_OWNER: TestAddress = TestAddress::new("agent_owner");
pub const CLIENT: TestAddress = TestAddress::new("client");
pub const WORKER: TestAddress = TestAddress::new("worker");
pub const VALIDATOR: TestAddress = TestAddress::new("validator");
pub const EMPLOYER: TestAddress = TestAddress::new("employer");
pub const AGENT: TestAddress = TestAddress::new("agent");

// ── SC Addresses ──
pub const IDENTITY_SC_ADDRESS: TestSCAddress = TestSCAddress::new("identity-registry");
pub const VALIDATION_SC_ADDRESS: TestSCAddress = TestSCAddress::new("validation-registry");
pub const REPUTATION_SC_ADDRESS: TestSCAddress = TestSCAddress::new("reputation-registry");
pub const ESCROW_SC_ADDRESS: TestSCAddress = TestSCAddress::new("escrow");

// ── Tokens ──
pub const AGENT_TOKEN: TestTokenIdentifier = TestTokenIdentifier::new("AGENT-abcdef");
pub const PAYMENT_TOKEN: TestTokenIdentifier = TestTokenIdentifier::new("USDC-abcdef");
pub const WRONG_TOKEN: TestTokenIdentifier = TestTokenIdentifier::new("WRONG-abcdef");

// ── NFT Roles ──
pub static NFT_ROLES: &[EsdtLocalRole] = &[
    EsdtLocalRole::NftCreate,
    EsdtLocalRole::Mint,
    EsdtLocalRole::NftBurn,
    EsdtLocalRole::NftUpdateAttributes,
    EsdtLocalRole::NftRecreate,
];

// ── Code Paths ──
pub const IDENTITY_CODE: MxscPath =
    MxscPath::new("../identity-registry/output/identity-registry.mxsc.json");
pub const VALIDATION_CODE: MxscPath =
    MxscPath::new("../validation-registry/output/validation-registry.mxsc.json");
pub const REPUTATION_CODE: MxscPath =
    MxscPath::new("../reputation-registry/output/reputation-registry.mxsc.json");
pub const ESCROW_CODE: MxscPath = MxscPath::new("../escrow/output/escrow.mxsc.json");
