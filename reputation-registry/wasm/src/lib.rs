#![no_std]

multiversx_sc_wasm_adapter::endpoints! {
    reputation_registry
    (
        init => init
        submit_feedback => submit_feedback
        reputation_score => reputation_score
        total_jobs => total_jobs
    )
}
