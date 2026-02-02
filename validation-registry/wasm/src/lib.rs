#![no_std]
#![cfg(not(test))]
#![cfg(not(test))]

#[cfg(not(test))]
multiversx_sc_wasm_adapter::allocator!();
#[cfg(not(test))]
multiversx_sc_wasm_adapter::panic_handler!();

#[cfg(not(test))]
multiversx_sc_wasm_adapter::endpoints! {
    validation_registry
    (
        init => init
        init_job => init_job
        submit_proof => submit_proof
        verify_job => verify_job
        clean_old_jobs => clean_old_jobs
        getJobStatus => job_status
        getJobProof => job_proof
        getJobEmployer => job_employer
        getJobCreationTimestamp => job_creation_timestamp
        getJobAgentNonce => job_agent_nonce
    )
}

#[cfg(not(test))]
multiversx_sc_wasm_adapter::async_callback_empty! {}
