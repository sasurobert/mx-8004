pub use common::errors::ERR_JOB_NOT_FOUND;
pub use common::errors::ERR_NOT_AGENT_OWNER;

pub const ERR_JOB_ALREADY_INITIALIZED: &str = "Job already initialized";
pub const ERR_INSUFFICIENT_PAYMENT: &str = "Insufficient payment";
pub const ERR_SERVICE_NOT_FOUND: &str = "Service config not found for agent";
pub const ERR_INVALID_PAYMENT: &str = "Invalid payment token";
pub const ERR_VALIDATION_REQUEST_NOT_FOUND: &str = "Validation request not found";
pub const ERR_NOT_VALIDATOR: &str = "Only the designated validator can respond";
pub const ERR_INVALID_AGENT_NFT: &str = "Invalid agent NFT: wrong token ID or nonce";
pub const ERR_JOB_STATUS_INVALID: &str = "Job status invalid for proof submission";
