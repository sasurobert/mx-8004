#![no_std]

multiversx_sc_wasm_adapter::endpoints! {
    identity_registry
    (
        init => init
        register_agent => register_agent
        update_agent => update_agent
        getAgent => get_agent
        agent_nft_token_id => agent_nft_token_id
        agent_uri => agent_uri
        agent_public_key => agent_public_key
    )
}
