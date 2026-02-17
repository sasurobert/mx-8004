use common::structs::{MetadataEntry, ServiceConfigInput};
use multiversx_sc::types::{BigUint, ManagedAddress, ManagedArgBuffer, ManagedBuffer, TokenId};
use multiversx_sc_scenario::imports::ExpectError;
use multiversx_sc_snippets::imports::*;
use proxies::{
    identity_registry_proxy::IdentityRegistryProxy,
    reputation_registry_proxy::ReputationRegistryProxy,
    validation_registry_proxy::ValidationRegistryProxy,
};

const GATEWAY: &str = "http://localhost:8085";

#[allow(dead_code)]
pub struct CsInteract {
    pub interactor: Interactor,
    pub owner: Address,
    pub agent_owner: Address,
    pub client: Address,
    pub worker: Address,
    pub identity_addr: Bech32Address,
    pub validation_addr: Bech32Address,
    pub reputation_addr: Bech32Address,
    pub agent_token_id: String,
    identity_code: BytesValue,
    validation_code: BytesValue,
    reputation_code: BytesValue,
}

impl CsInteract {
    pub async fn new() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await.use_chain_simulator(true);

        interactor.set_current_dir_from_workspace("tests");

        let owner = interactor.register_wallet(test_wallets::alice()).await;
        let agent_owner = interactor.register_wallet(test_wallets::bob()).await;
        let client = interactor.register_wallet(test_wallets::carol()).await;
        let worker = interactor.register_wallet(test_wallets::dan()).await;

        interactor.generate_blocks_until_all_activations().await;

        let identity_code = BytesValue::interpret_from(
            "mxsc:../identity-registry/output/identity-registry.mxsc.json",
            &InterpreterContext::default(),
        );
        let validation_code = BytesValue::interpret_from(
            "mxsc:../validation-registry/output/validation-registry.mxsc.json",
            &InterpreterContext::default(),
        );
        let reputation_code = BytesValue::interpret_from(
            "mxsc:../reputation-registry/output/reputation-registry.mxsc.json",
            &InterpreterContext::default(),
        );

        // Deploy identity-registry
        let identity_addr = interactor
            .tx()
            .from(&owner)
            .gas(80_000_000u64)
            .typed(IdentityRegistryProxy)
            .init()
            .code(&identity_code)
            .returns(ReturnsNewBech32Address)
            .run()
            .await;

        println!("Identity deployed: {identity_addr}");

        // Issue NFT token
        interactor
            .tx()
            .from(&owner)
            .to(&identity_addr)
            .gas(80_000_000u64)
            .typed(IdentityRegistryProxy)
            .issue_token(
                ManagedBuffer::<StaticApi>::from("AgentNFT"),
                ManagedBuffer::<StaticApi>::from("AGENT"),
            )
            .egld(BigUint::<StaticApi>::from(50_000_000_000_000_000u64))
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        // Wait for async issuance callback
        let _ = interactor.generate_blocks(3).await;

        // Query the issued token ID
        let agent_token_id = interactor
            .query()
            .to(&identity_addr)
            .typed(IdentityRegistryProxy)
            .agent_token_id()
            .returns(ReturnsResultUnmanaged)
            .run()
            .await
            .to_string();

        println!("Agent token ID: {agent_token_id}");
        assert!(!agent_token_id.is_empty(), "Token issuance failed");

        // Deploy validation-registry
        let validation_addr = interactor
            .tx()
            .from(&owner)
            .gas(80_000_000u64)
            .typed(ValidationRegistryProxy)
            .init(&identity_addr)
            .code(&validation_code)
            .returns(ReturnsNewBech32Address)
            .run()
            .await;

        println!("Validation deployed: {validation_addr}");

        // Deploy reputation-registry
        let reputation_addr = interactor
            .tx()
            .from(&owner)
            .gas(80_000_000u64)
            .typed(ReputationRegistryProxy)
            .init(&validation_addr, &identity_addr)
            .code(&reputation_code)
            .returns(ReturnsNewBech32Address)
            .run()
            .await;

        println!("Reputation deployed: {reputation_addr}");

        Self {
            interactor,
            owner,
            agent_owner,
            client,
            worker,
            identity_addr,
            validation_addr,
            reputation_addr,
            agent_token_id,
            identity_code,
            validation_code,
            reputation_code,
        }
    }

    // ── Register Agent (raw call for Counted encoding) ──

    pub async fn register_agent(&mut self, from: &Address, name: &[u8], uri: &[u8], pubkey: &[u8]) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(ManagedBuffer::<StaticApi>::from(name));
        args.push_arg(ManagedBuffer::<StaticApi>::from(uri));
        args.push_arg(ManagedBuffer::<StaticApi>::from(pubkey));
        // Counted metadata: 0
        args.push_arg(0u32);
        // Counted services: 0
        args.push_arg(0u32);

        self.interactor
            .tx()
            .from(from)
            .to(&self.identity_addr)
            .gas(30_000_000u64)
            .raw_call("register_agent")
            .arguments_raw(args)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    pub async fn register_agent_with_meta(
        &mut self,
        from: &Address,
        name: &[u8],
        uri: &[u8],
        pubkey: &[u8],
        metadata: &[(&[u8], &[u8])],
        services: &[(u32, u64, &[u8], u64)],
    ) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(ManagedBuffer::<StaticApi>::from(name));
        args.push_arg(ManagedBuffer::<StaticApi>::from(uri));
        args.push_arg(ManagedBuffer::<StaticApi>::from(pubkey));
        args.push_arg(metadata.len());
        for (k, v) in metadata {
            args.push_arg(MetadataEntry::<StaticApi> {
                key: ManagedBuffer::from(*k),
                value: ManagedBuffer::from(*v),
            });
        }
        args.push_arg(services.len());
        for (sid, price, token, nonce) in services {
            args.push_arg(ServiceConfigInput::<StaticApi> {
                service_id: *sid,
                price: BigUint::from(*price),
                token: TokenId::from(*token),
                nonce: *nonce,
            });
        }

        self.interactor
            .tx()
            .from(from)
            .to(&self.identity_addr)
            .gas(30_000_000u64)
            .raw_call("register_agent")
            .arguments_raw(args)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    // ── Validation Registry ──

    pub async fn init_job(&mut self, from: &Address, job_id: &[u8], agent_nonce: u64) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .init_job(
                ManagedBuffer::<StaticApi>::from(job_id),
                agent_nonce,
                OptionalValue::<u32>::None,
            )
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    pub async fn init_job_with_payment(
        &mut self,
        from: &Address,
        job_id: &[u8],
        agent_nonce: u64,
        service_id: u32,
        token: &str,
        amount: u64,
    ) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .init_job(
                ManagedBuffer::<StaticApi>::from(job_id),
                agent_nonce,
                OptionalValue::Some(service_id),
            )
            .payment((
                EsdtTokenIdentifier::from(token),
                0u64,
                BigUint::<StaticApi>::from(amount),
            ))
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    /// Calls init_job with a service_id but NO payment (for free services with price=0).
    pub async fn init_job_with_free_service(
        &mut self,
        from: &Address,
        job_id: &[u8],
        agent_nonce: u64,
        service_id: u32,
    ) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .init_job(
                ManagedBuffer::<StaticApi>::from(job_id),
                agent_nonce,
                OptionalValue::Some(service_id),
            )
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    pub async fn submit_proof(&mut self, from: &Address, job_id: &[u8], proof: &[u8]) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .submit_proof(
                ManagedBuffer::<StaticApi>::from(job_id),
                ManagedBuffer::<StaticApi>::from(proof),
            )
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    pub async fn validation_request(
        &mut self,
        from: &Address,
        job_id: &[u8],
        validator: &Address,
        request_uri: &[u8],
        request_hash: &[u8],
    ) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .validation_request(
                ManagedBuffer::<StaticApi>::from(job_id),
                ManagedAddress::<StaticApi>::from(validator),
                ManagedBuffer::<StaticApi>::from(request_uri),
                ManagedBuffer::<StaticApi>::from(request_hash),
            )
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    pub async fn validation_request_expect_err(
        &mut self,
        from: &Address,
        job_id: &[u8],
        request_uri: &[u8],
        request_hash: &[u8],
        err_code: u64,
        err_msg: &str,
    ) {
        let validator = self.owner.clone();
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .validation_request(
                ManagedBuffer::<StaticApi>::from(job_id),
                ManagedAddress::<StaticApi>::from(&validator),
                ManagedBuffer::<StaticApi>::from(request_uri),
                ManagedBuffer::<StaticApi>::from(request_hash),
            )
            .returns(ExpectError(err_code, err_msg))
            .run()
            .await;
    }

    pub async fn validation_response(
        &mut self,
        from: &Address,
        request_hash: &[u8],
        response: u8,
        response_uri: &[u8],
        response_hash: &[u8],
        tag: &[u8],
    ) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .validation_response(
                ManagedBuffer::<StaticApi>::from(request_hash),
                response,
                ManagedBuffer::<StaticApi>::from(response_uri),
                ManagedBuffer::<StaticApi>::from(response_hash),
                ManagedBuffer::<StaticApi>::from(tag),
            )
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    // ── Reputation Registry ──

    pub async fn give_feedback_simple(
        &mut self,
        from: &Address,
        job_id: &[u8],
        agent_nonce: u64,
        rating: u64,
    ) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.reputation_addr)
            .gas(30_000_000u64)
            .typed(ReputationRegistryProxy)
            .give_feedback_simple(
                ManagedBuffer::<StaticApi>::from(job_id),
                agent_nonce,
                BigUint::<StaticApi>::from(rating),
            )
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    pub async fn append_response(&mut self, from: &Address, job_id: &[u8], uri: &[u8]) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.reputation_addr)
            .gas(30_000_000u64)
            .typed(ReputationRegistryProxy)
            .append_response(
                ManagedBuffer::<StaticApi>::from(job_id),
                ManagedBuffer::<StaticApi>::from(uri),
            )
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;
    }

    // ── Queries ──

    pub async fn query_agent_token_id(&mut self) -> String {
        self.interactor
            .query()
            .to(&self.identity_addr)
            .typed(IdentityRegistryProxy)
            .agent_token_id()
            .returns(ReturnsResultUnmanaged)
            .run()
            .await
            .to_string()
    }

    pub async fn query_is_job_verified(&mut self, job_id: &[u8]) -> bool {
        self.interactor
            .query()
            .to(&self.validation_addr)
            .typed(ValidationRegistryProxy)
            .is_job_verified(ManagedBuffer::<StaticApi>::from(job_id))
            .returns(ReturnsResultUnmanaged)
            .run()
            .await
    }

    pub async fn query_reputation_score(&mut self, agent_nonce: u64) -> RustBigUint {
        self.interactor
            .query()
            .to(&self.reputation_addr)
            .typed(ReputationRegistryProxy)
            .reputation_score(agent_nonce)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await
    }

    pub async fn query_total_jobs(&mut self, agent_nonce: u64) -> u64 {
        self.interactor
            .query()
            .to(&self.reputation_addr)
            .typed(ReputationRegistryProxy)
            .total_jobs(agent_nonce)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await
    }

    pub async fn query_has_given_feedback(&mut self, job_id: &[u8]) -> bool {
        self.interactor
            .query()
            .to(&self.reputation_addr)
            .typed(ReputationRegistryProxy)
            .has_given_feedback(ManagedBuffer::<StaticApi>::from(job_id))
            .returns(ReturnsResultUnmanaged)
            .run()
            .await
    }

    // ── Error-path helpers ──

    pub async fn register_agent_expect_err(
        &mut self,
        from: &Address,
        name: &[u8],
        uri: &[u8],
        pubkey: &[u8],
        err_code: u64,
        err_msg: &str,
    ) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(ManagedBuffer::<StaticApi>::from(name));
        args.push_arg(ManagedBuffer::<StaticApi>::from(uri));
        args.push_arg(ManagedBuffer::<StaticApi>::from(pubkey));
        args.push_arg(0u32); // metadata count
        args.push_arg(0u32); // services count

        self.interactor
            .tx()
            .from(from)
            .to(&self.identity_addr)
            .gas(30_000_000u64)
            .raw_call("register_agent")
            .arguments_raw(args)
            .returns(ExpectError(err_code, err_msg))
            .run()
            .await;
    }

    pub async fn submit_proof_expect_err(
        &mut self,
        from: &Address,
        job_id: &[u8],
        proof: &[u8],
        err_code: u64,
        err_msg: &str,
    ) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .submit_proof(
                ManagedBuffer::<StaticApi>::from(job_id),
                ManagedBuffer::<StaticApi>::from(proof),
            )
            .returns(ExpectError(err_code, err_msg))
            .run()
            .await;
    }

    pub async fn init_job_expect_err(
        &mut self,
        from: &Address,
        job_id: &[u8],
        agent_nonce: u64,
        err_code: u64,
        err_msg: &str,
    ) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .init_job(
                ManagedBuffer::<StaticApi>::from(job_id),
                agent_nonce,
                OptionalValue::<u32>::None,
            )
            .returns(ExpectError(err_code, err_msg))
            .run()
            .await;
    }

    /// Calls init_job with a service_id but NO payment — expects error.
    pub async fn init_job_with_free_service_expect_err(
        &mut self,
        from: &Address,
        job_id: &[u8],
        agent_nonce: u64,
        service_id: u32,
        err_code: u64,
        err_msg: &str,
    ) {
        self.interactor
            .tx()
            .from(from)
            .to(&self.validation_addr)
            .gas(30_000_000u64)
            .typed(ValidationRegistryProxy)
            .init_job(
                ManagedBuffer::<StaticApi>::from(job_id),
                agent_nonce,
                OptionalValue::Some(service_id),
            )
            .returns(ExpectError(err_code, err_msg))
            .run()
            .await;
    }

    pub async fn issue_token_expect_err(&mut self, err_code: u64, err_msg: &str) {
        self.interactor
            .tx()
            .from(&self.owner)
            .to(&self.identity_addr)
            .gas(80_000_000u64)
            .typed(IdentityRegistryProxy)
            .issue_token(
                ManagedBuffer::<StaticApi>::from("AgentNFT"),
                ManagedBuffer::<StaticApi>::from("AGENT"),
            )
            .egld(BigUint::<StaticApi>::from(50_000_000_000_000_000u64))
            .returns(ExpectError(err_code, err_msg))
            .run()
            .await;
    }
}
