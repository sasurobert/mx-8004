#![no_std]

multiversx_sc_wasm_adapter::endpoints! {
    validation_registry
    (
        init => init
        submit_proof => submit_proof
        verify_job => verify_job
        clean_old_jobs => clean_old_jobs
        is_job_verified => is_job_verified
        getJobProof => job_proof
        getJobStatus => job_status
        getJobEmployer => job_employer
        getJobCreationTimestamp => job_creation_timestamp
        getJobAgentNonce => job_agent_nonce
    )
}
