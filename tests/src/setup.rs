use crate::constants::*;
use common::structs::{AgentDetails, JobData, MetadataEntry, ServiceConfigInput};
use identity_registry::storage::StorageModule;
use multiversx_sc::proxy_imports::MultiValue2;
use multiversx_sc::proxy_imports::OptionalValue;
use multiversx_sc::types::{
    BigUint, EgldOrEsdtTokenPayment, EsdtTokenIdentifier, ManagedAddress, ManagedArgBuffer,
    ManagedBuffer, MultiValueEncoded, ReturnsNewManagedAddress, ReturnsResult, TestEsdtTransfer,
    TokenId,
};
use multiversx_sc_scenario::{
    ScenarioTxRun, ScenarioTxWhitebox, ScenarioWorld, api::StaticApi, imports::ExpectMessage,
};
use proxies::{
    identity_registry_proxy::IdentityRegistryProxy,
    reputation_registry_proxy::ReputationRegistryProxy,
    validation_registry_proxy::ValidationRegistryProxy,
};
use validation_registry::storage::ExternalStorageModule;

pub fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();
    blockchain.register_contract(IDENTITY_CODE, identity_registry::ContractBuilder);
    blockchain.register_contract(VALIDATION_CODE, validation_registry::ContractBuilder);
    blockchain.register_contract(REPUTATION_CODE, reputation_registry::ContractBuilder);
    blockchain
}

pub struct AgentTestState {
    pub world: ScenarioWorld,
    pub identity_sc: ManagedAddress<StaticApi>,
    pub validation_sc: ManagedAddress<StaticApi>,
    pub reputation_sc: ManagedAddress<StaticApi>,
}

impl Default for AgentTestState {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentTestState {
    pub fn new() -> Self {
        let mut world = world();

        world
            .account(OWNER_ADDRESS)
            .nonce(1)
            .balance(100_000_000_000_000_000u64);

        let identity_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(IdentityRegistryProxy)
            .init()
            .code(IDENTITY_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(IDENTITY_SC_ADDRESS)
            .run();

        world
            .tx()
            .from(OWNER_ADDRESS)
            .to(IDENTITY_SC_ADDRESS)
            .whitebox(identity_registry::contract_obj, |sc| {
                sc.agent_token_id()
                    .set_token_id(AGENT_TOKEN.to_token_identifier());
            });

        world.set_esdt_local_roles(IDENTITY_SC_ADDRESS, AGENT_TOKEN.as_bytes(), NFT_ROLES);

        let validation_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(ValidationRegistryProxy)
            .init(identity_sc.clone())
            .code(VALIDATION_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(VALIDATION_SC_ADDRESS)
            .run();

        let reputation_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(ReputationRegistryProxy)
            .init(validation_sc.clone(), identity_sc.clone())
            .code(REPUTATION_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(REPUTATION_SC_ADDRESS)
            .run();

        world.account(AGENT_OWNER).nonce(1).balance(1_000_000u64);
        world
            .account(CLIENT)
            .nonce(1)
            .balance(1_000_000u64)
            .esdt_balance(PAYMENT_TOKEN, 1_000_000_000u64)
            .esdt_balance(WRONG_TOKEN, 1_000_000_000u64);
        world.account(WORKER).nonce(1).balance(1_000_000u64);
        world.account(VALIDATOR).nonce(1).balance(1_000_000u64);

        Self {
            world,
            identity_sc,
            validation_sc,
            reputation_sc,
        }
    }

    /// Create test state with identity registry deployed but WITHOUT issuing the agent token.
    /// Used to test the `register_agent` error path when token is not issued.
    pub fn new_no_token() -> Self {
        let mut world = world();

        world
            .account(OWNER_ADDRESS)
            .nonce(1)
            .balance(100_000_000_000_000_000u64);

        let identity_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(IdentityRegistryProxy)
            .init()
            .code(IDENTITY_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(IDENTITY_SC_ADDRESS)
            .run();

        // NOTE: Deliberately NOT setting agent_token_id — this is the "token not issued" state

        let validation_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(ValidationRegistryProxy)
            .init(identity_sc.clone())
            .code(VALIDATION_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(VALIDATION_SC_ADDRESS)
            .run();

        let reputation_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(ReputationRegistryProxy)
            .init(validation_sc.clone(), identity_sc.clone())
            .code(REPUTATION_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(REPUTATION_SC_ADDRESS)
            .run();

        world.account(AGENT_OWNER).nonce(1).balance(1_000_000u64);
        world.account(CLIENT).nonce(1).balance(1_000_000u64);

        Self {
            world,
            identity_sc,
            validation_sc,
            reputation_sc,
        }
    }

    // ── Raw call helpers for Counted multi-value endpoints ──

    /// Build raw args for registerAgent: name, uri, pubkey, count_meta, meta..., count_svc, svc...
    fn register_agent_raw_args(
        name: &[u8],
        uri: &[u8],
        pubkey: &[u8],
        metadata: &[(&[u8], &[u8])],
        services: &[(u32, u64, &[u8], u64)],
    ) -> ManagedArgBuffer<StaticApi> {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(ManagedBuffer::<StaticApi>::from(name));
        args.push_arg(ManagedBuffer::<StaticApi>::from(uri));
        args.push_arg(ManagedBuffer::<StaticApi>::from(pubkey));
        // Counted metadata
        args.push_arg(metadata.len());
        for (k, v) in metadata {
            args.push_arg(MetadataEntry::<StaticApi> {
                key: ManagedBuffer::from(*k),
                value: ManagedBuffer::from(*v),
            });
        }
        // Counted services
        args.push_arg(services.len());
        for (sid, price, token, nonce) in services {
            args.push_arg(ServiceConfigInput::<StaticApi> {
                service_id: *sid,
                price: BigUint::from(*price),
                token: TokenId::from(*token),
                nonce: *nonce,
            });
        }
        args
    }

    pub fn register_agent(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        name: &[u8],
        uri: &[u8],
        pubkey: &[u8],
        metadata: Vec<(&[u8], &[u8])>,
        services: Vec<(u32, u64, &[u8], u64)>,
    ) {
        let args = Self::register_agent_raw_args(name, uri, pubkey, &metadata, &services);
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .raw_call("register_agent")
            .arguments_raw(args)
            .run();
    }

    pub fn register_agent_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        name: &[u8],
        uri: &[u8],
        pubkey: &[u8],
        err_msg: &str,
    ) {
        let args = Self::register_agent_raw_args(name, uri, pubkey, &[], &[]);
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .raw_call("register_agent")
            .arguments_raw(args)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn set_metadata(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nonce: u64,
        entries: Vec<(&[u8], &[u8])>,
    ) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(nonce);
        args.push_arg(entries.len());
        for (k, v) in &entries {
            args.push_arg(MetadataEntry::<StaticApi> {
                key: ManagedBuffer::from(*k),
                value: ManagedBuffer::from(*v),
            });
        }
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .raw_call("set_metadata")
            .arguments_raw(args)
            .run();
    }

    pub fn remove_metadata(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nonce: u64,
        keys: Vec<&[u8]>,
    ) {
        let mut keys_encoded = MultiValueEncoded::<StaticApi, ManagedBuffer<StaticApi>>::new();
        for k in &keys {
            keys_encoded.push(ManagedBuffer::from(*k));
        }
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .remove_metadata(nonce, keys_encoded)
            .run();
    }

    pub fn set_service_configs(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nonce: u64,
        services: Vec<(u32, u64, &[u8], u64)>,
    ) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(nonce);
        args.push_arg(services.len());
        for (sid, price, token, tok_nonce) in &services {
            args.push_arg(ServiceConfigInput::<StaticApi> {
                service_id: *sid,
                price: BigUint::from(*price),
                token: TokenId::from(*token),
                nonce: *tok_nonce,
            });
        }
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .raw_call("set_service_configs")
            .arguments_raw(args)
            .run();
    }

    pub fn remove_service_configs(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nonce: u64,
        service_ids: Vec<u32>,
    ) {
        let mut ids_encoded = MultiValueEncoded::<StaticApi, u32>::new();
        for sid in &service_ids {
            ids_encoded.push(*sid);
        }
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .remove_service_configs(nonce, ids_encoded)
            .run();
    }

    // ── Validation Registry ──

    pub fn init_job(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        agent_nonce: u64,
        service_id: Option<u32>,
    ) {
        let svc = match service_id {
            Some(sid) => OptionalValue::Some(sid),
            None => OptionalValue::None,
        };
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .init_job(ManagedBuffer::from(job_id), agent_nonce, svc)
            .run();
    }

    pub fn init_job_with_payment(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        agent_nonce: u64,
        service_id: u32,
        token: &str,
        token_nonce: u64,
        amount: u64,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .init_job(
                ManagedBuffer::from(job_id),
                agent_nonce,
                OptionalValue::Some(service_id),
            )
            .esdt(TestEsdtTransfer(
                multiversx_sc_scenario::imports::TestTokenIdentifier::new(token),
                token_nonce,
                amount,
            ))
            .run();
    }

    pub fn init_job_with_payment_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        agent_nonce: u64,
        service_id: u32,
        token: &str,
        token_nonce: u64,
        amount: u64,
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .init_job(
                ManagedBuffer::from(job_id),
                agent_nonce,
                OptionalValue::Some(service_id),
            )
            .esdt(TestEsdtTransfer(
                multiversx_sc_scenario::imports::TestTokenIdentifier::new(token),
                token_nonce,
                amount,
            ))
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn submit_proof(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        proof: &[u8],
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .submit_proof(ManagedBuffer::from(job_id), ManagedBuffer::from(proof))
            .run();
    }

    pub fn submit_proof_with_nft(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        proof: &[u8],
        token_id: &multiversx_sc::types::TestTokenIdentifier,
        nonce: u64,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .submit_proof_with_nft(ManagedBuffer::from(job_id), ManagedBuffer::from(proof))
            .single_esdt(
                &EsdtTokenIdentifier::from(*token_id),
                nonce,
                &BigUint::from(1u64),
            )
            .run();
    }

    pub fn submit_proof_with_nft_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        proof: &[u8],
        token_id: &multiversx_sc::types::TestTokenIdentifier,
        nonce: u64,
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .submit_proof_with_nft(ManagedBuffer::from(job_id), ManagedBuffer::from(proof))
            .single_esdt(
                &EsdtTokenIdentifier::from(*token_id),
                nonce,
                &BigUint::from(1u64),
            )
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn validation_request(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        validator: &multiversx_sc::types::TestAddress,
        request_uri: &[u8],
        request_hash: &[u8],
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .validation_request(
                ManagedBuffer::from(job_id),
                validator.to_managed_address(),
                ManagedBuffer::from(request_uri),
                ManagedBuffer::from(request_hash),
            )
            .run();
    }

    pub fn validation_request_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        validator: &multiversx_sc::types::TestAddress,
        request_uri: &[u8],
        request_hash: &[u8],
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .validation_request(
                ManagedBuffer::from(job_id),
                validator.to_managed_address(),
                ManagedBuffer::from(request_uri),
                ManagedBuffer::from(request_hash),
            )
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn validation_response(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        request_hash: &[u8],
        response: u8,
        response_uri: &[u8],
        response_hash: &[u8],
        tag: &[u8],
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .validation_response(
                ManagedBuffer::from(request_hash),
                response,
                ManagedBuffer::from(response_uri),
                ManagedBuffer::from(response_hash),
                ManagedBuffer::from(tag),
            )
            .run();
    }

    pub fn validation_response_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        request_hash: &[u8],
        response: u8,
        response_uri: &[u8],
        response_hash: &[u8],
        tag: &[u8],
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .validation_response(
                ManagedBuffer::from(request_hash),
                response,
                ManagedBuffer::from(response_uri),
                ManagedBuffer::from(response_hash),
                ManagedBuffer::from(tag),
            )
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn clean_old_jobs(&mut self, job_ids: Vec<&[u8]>) {
        let mut ids_encoded = MultiValueEncoded::<StaticApi, ManagedBuffer<StaticApi>>::new();
        for id in &job_ids {
            ids_encoded.push(ManagedBuffer::from(*id));
        }
        self.world
            .tx()
            .from(OWNER_ADDRESS)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .clean_old_jobs(ids_encoded)
            .run();
    }

    // ── Reputation Registry ──

    pub fn give_feedback_simple(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        agent_nonce: u64,
        rating: u64,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .give_feedback_simple(
                ManagedBuffer::from(job_id),
                agent_nonce,
                BigUint::from(rating),
            )
            .run();
    }

    pub fn give_feedback_simple_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        agent_nonce: u64,
        rating: u64,
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .give_feedback_simple(
                ManagedBuffer::from(job_id),
                agent_nonce,
                BigUint::from(rating),
            )
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn append_response(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        response_uri: &[u8],
    ) {
        self.world
            .tx()
            .from(*from)
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .append_response(
                ManagedBuffer::from(job_id),
                ManagedBuffer::from(response_uri),
            )
            .run();
    }

    // ── Queries ──

    pub fn query_agent_details(&mut self, nonce: u64) -> AgentDetails<StaticApi> {
        self.world
            .query()
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .agent_details(nonce)
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_agent_owner(&mut self, nonce: u64) -> ManagedAddress<StaticApi> {
        self.world
            .query()
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .get_agent_owner(nonce)
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_metadata(
        &mut self,
        nonce: u64,
        key: &[u8],
    ) -> OptionalValue<ManagedBuffer<StaticApi>> {
        self.world
            .query()
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .get_metadata(nonce, ManagedBuffer::from(key))
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_service_config(
        &mut self,
        nonce: u64,
        service_id: u32,
    ) -> OptionalValue<EgldOrEsdtTokenPayment<StaticApi>> {
        self.world
            .query()
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .get_agent_service_config(nonce, service_id)
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_is_job_verified(&mut self, job_id: &[u8]) -> bool {
        self.world
            .query()
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .is_job_verified(ManagedBuffer::from(job_id))
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_job_data(&mut self, job_id: &[u8]) -> OptionalValue<JobData<StaticApi>> {
        self.world
            .query()
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .get_job_data(ManagedBuffer::from(job_id))
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_reputation_score(&mut self, agent_nonce: u64) -> BigUint<StaticApi> {
        self.world
            .query()
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .reputation_score(agent_nonce)
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_total_jobs(&mut self, agent_nonce: u64) -> u64 {
        self.world
            .query()
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .total_jobs(agent_nonce)
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_has_given_feedback(&mut self, job_id: &[u8]) -> bool {
        self.world
            .query()
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .has_given_feedback(ManagedBuffer::from(job_id))
            .returns(ReturnsResult)
            .run()
    }

    // query_is_feedback_authorized removed — ERC-8004 compliance

    pub fn query_agent_response(&mut self, job_id: &[u8]) -> ManagedBuffer<StaticApi> {
        self.world
            .query()
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .agent_response(ManagedBuffer::from(job_id))
            .returns(ReturnsResult)
            .run()
    }

    // ── Upgrade helpers ──

    pub fn upgrade_identity(&mut self) {
        self.world
            .tx()
            .from(OWNER_ADDRESS)
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .upgrade()
            .code(IDENTITY_CODE)
            .run();
    }

    pub fn upgrade_validation(&mut self) {
        self.world
            .tx()
            .from(OWNER_ADDRESS)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .upgrade()
            .code(VALIDATION_CODE)
            .run();
    }

    pub fn upgrade_reputation(&mut self) {
        self.world
            .tx()
            .from(OWNER_ADDRESS)
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .upgrade()
            .code(REPUTATION_CODE)
            .run();
    }

    // ── Issue token (already issued error path) ──

    pub fn issue_token_expect_err(&mut self, err_msg: &str) {
        self.world
            .tx()
            .from(OWNER_ADDRESS)
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .issue_token(
                ManagedBuffer::from(b"AgentNFT"),
                ManagedBuffer::from(b"AGENT"),
            )
            .egld(50_000_000_000_000_000u64)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    // ── Update agent (raw call with NFT transfer) ──

    pub fn update_agent_raw(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nft_nonce: u64,
        new_name: &[u8],
        new_uri: &[u8],
        new_public_key: &[u8],
        metadata: Option<Vec<(&[u8], &[u8])>>,
        services: Option<Vec<(u32, u64, &[u8], u64)>>,
    ) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(ManagedBuffer::<StaticApi>::from(new_name));
        args.push_arg(ManagedBuffer::<StaticApi>::from(new_uri));
        args.push_arg(ManagedBuffer::<StaticApi>::from(new_public_key));

        // Optional metadata (OptionalValue<MultiValueEncodedCounted>)
        if let Some(meta) = metadata {
            args.push_arg(meta.len());
            for (k, v) in &meta {
                args.push_arg(MetadataEntry::<StaticApi> {
                    key: ManagedBuffer::from(*k),
                    value: ManagedBuffer::from(*v),
                });
            }
        }

        // Optional services
        if let Some(svcs) = services {
            args.push_arg(svcs.len());
            for (sid, price, token, tok_nonce) in &svcs {
                args.push_arg(ServiceConfigInput::<StaticApi> {
                    service_id: *sid,
                    price: BigUint::from(*price),
                    token: TokenId::from(*token),
                    nonce: *tok_nonce,
                });
            }
        }

        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .raw_call("update_agent")
            .arguments_raw(args)
            .esdt(TestEsdtTransfer(AGENT_TOKEN, nft_nonce, 1))
            .run();
    }

    pub fn update_agent_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nft_nonce: u64,
        new_name: &[u8],
        new_uri: &[u8],
        new_public_key: &[u8],
        err_msg: &str,
    ) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(ManagedBuffer::<StaticApi>::from(new_name));
        args.push_arg(ManagedBuffer::<StaticApi>::from(new_uri));
        args.push_arg(ManagedBuffer::<StaticApi>::from(new_public_key));

        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .raw_call("update_agent")
            .arguments_raw(args)
            .esdt(TestEsdtTransfer(AGENT_TOKEN, nft_nonce, 1))
            .returns(ExpectMessage(err_msg))
            .run();
    }

    // ── Admin config setters ──

    pub fn set_identity_registry_address(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        address: ManagedAddress<StaticApi>,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .set_identity_registry_address(address)
            .run();
    }

    pub fn set_identity_registry_address_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        address: ManagedAddress<StaticApi>,
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .set_identity_registry_address(address)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn set_reputation_identity_address(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        address: ManagedAddress<StaticApi>,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .set_identity_contract_address(address)
            .run();
    }

    pub fn set_reputation_identity_address_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        address: ManagedAddress<StaticApi>,
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .set_identity_contract_address(address)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn set_reputation_validation_address(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        address: ManagedAddress<StaticApi>,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .set_validation_contract_address(address)
            .run();
    }

    pub fn set_reputation_validation_address_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        address: ManagedAddress<StaticApi>,
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .set_validation_contract_address(address)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    // ── Error-path helpers ──

    pub fn init_job_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        agent_nonce: u64,
        service_id: Option<u32>,
        err_msg: &str,
    ) {
        let svc = match service_id {
            Some(sid) => OptionalValue::Some(sid),
            None => OptionalValue::None,
        };
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .init_job(ManagedBuffer::from(job_id), agent_nonce, svc)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn submit_proof_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        proof: &[u8],
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .submit_proof(ManagedBuffer::from(job_id), ManagedBuffer::from(proof))
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn set_metadata_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nonce: u64,
        entries: Vec<(&[u8], &[u8])>,
        err_msg: &str,
    ) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(nonce);
        args.push_arg(entries.len());
        for (k, v) in &entries {
            args.push_arg(MetadataEntry::<StaticApi> {
                key: ManagedBuffer::from(*k),
                value: ManagedBuffer::from(*v),
            });
        }
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .raw_call("set_metadata")
            .arguments_raw(args)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn set_service_configs_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nonce: u64,
        services: Vec<(u32, u64, &[u8], u64)>,
        err_msg: &str,
    ) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(nonce);
        args.push_arg(services.len());
        for (sid, price, token, tok_nonce) in &services {
            args.push_arg(ServiceConfigInput::<StaticApi> {
                service_id: *sid,
                price: BigUint::from(*price),
                token: TokenId::from(*token),
                nonce: *tok_nonce,
            });
        }
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .raw_call("set_service_configs")
            .arguments_raw(args)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn remove_metadata_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nonce: u64,
        keys: Vec<&[u8]>,
        err_msg: &str,
    ) {
        let mut keys_encoded = MultiValueEncoded::<StaticApi, ManagedBuffer<StaticApi>>::new();
        for k in &keys {
            keys_encoded.push(ManagedBuffer::from(*k));
        }
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .remove_metadata(nonce, keys_encoded)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn remove_service_configs_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        nonce: u64,
        service_ids: Vec<u32>,
        err_msg: &str,
    ) {
        let mut ids_encoded = MultiValueEncoded::<StaticApi, u32>::new();
        for sid in &service_ids {
            ids_encoded.push(*sid);
        }
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .remove_service_configs(nonce, ids_encoded)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn append_response_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        response_uri: &[u8],
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .append_response(
                ManagedBuffer::from(job_id),
                ManagedBuffer::from(response_uri),
            )
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn init_job_with_wrong_token_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        agent_nonce: u64,
        service_id: u32,
        token: &str,
        token_nonce: u64,
        amount: u64,
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .init_job(
                ManagedBuffer::from(job_id),
                agent_nonce,
                OptionalValue::Some(service_id),
            )
            .esdt(TestEsdtTransfer(
                multiversx_sc_scenario::imports::TestTokenIdentifier::new(token),
                token_nonce,
                amount,
            ))
            .returns(ExpectMessage(err_msg))
            .run();
    }

    // ── Additional query helpers ──

    pub fn query_agent_token_id(&mut self) -> EsdtTokenIdentifier<StaticApi> {
        self.world
            .query()
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .agent_token_id()
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_agents(
        &mut self,
    ) -> MultiValueEncoded<StaticApi, MultiValue2<u64, ManagedAddress<StaticApi>>> {
        self.world
            .query()
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .agents()
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_agent(&mut self, nonce: u64) -> AgentDetails<StaticApi> {
        self.world
            .query()
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .get_agent(nonce)
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_agent_metadata_bulk(
        &mut self,
        nonce: u64,
    ) -> MultiValueEncoded<StaticApi, MultiValue2<ManagedBuffer<StaticApi>, ManagedBuffer<StaticApi>>>
    {
        self.world
            .query()
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .agent_metadata(nonce)
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_agent_service_bulk(
        &mut self,
        nonce: u64,
    ) -> MultiValueEncoded<StaticApi, MultiValue2<u32, multiversx_sc::types::Payment<StaticApi>>>
    {
        self.world
            .query()
            .to(IDENTITY_SC_ADDRESS)
            .typed(IdentityRegistryProxy)
            .agent_service_config(nonce)
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_validation_contract_address(&mut self) -> ManagedAddress<StaticApi> {
        self.world
            .query()
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .validation_contract_address()
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_identity_contract_address(&mut self) -> ManagedAddress<StaticApi> {
        self.world
            .query()
            .to(REPUTATION_SC_ADDRESS)
            .typed(ReputationRegistryProxy)
            .identity_contract_address()
            .returns(ReturnsResult)
            .run()
    }
}

// ════════════════════════════════════════════════════════════
// Escrow Test State — extends AgentTestState with Escrow SC
// ════════════════════════════════════════════════════════════

use escrow::storage::EscrowData;
use proxies::escrow_proxy::EscrowProxy;

pub struct EscrowTestState {
    pub world: ScenarioWorld,
    pub identity_sc: ManagedAddress<StaticApi>,
    pub validation_sc: ManagedAddress<StaticApi>,
    pub reputation_sc: ManagedAddress<StaticApi>,
    pub escrow_sc: ManagedAddress<StaticApi>,
}

impl Default for EscrowTestState {
    fn default() -> Self {
        Self::new()
    }
}

impl EscrowTestState {
    pub fn new() -> Self {
        let mut blockchain = ScenarioWorld::new();
        blockchain.register_contract(IDENTITY_CODE, identity_registry::ContractBuilder);
        blockchain.register_contract(VALIDATION_CODE, validation_registry::ContractBuilder);
        blockchain.register_contract(REPUTATION_CODE, reputation_registry::ContractBuilder);
        blockchain.register_contract(ESCROW_CODE, escrow::ContractBuilder);

        let mut world = blockchain;

        world
            .account(OWNER_ADDRESS)
            .nonce(1)
            .balance(100_000_000_000_000_000u64);

        let identity_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(IdentityRegistryProxy)
            .init()
            .code(IDENTITY_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(IDENTITY_SC_ADDRESS)
            .run();

        world
            .tx()
            .from(OWNER_ADDRESS)
            .to(IDENTITY_SC_ADDRESS)
            .whitebox(identity_registry::contract_obj, |sc| {
                sc.agent_token_id()
                    .set_token_id(AGENT_TOKEN.to_token_identifier());
            });

        world.set_esdt_local_roles(IDENTITY_SC_ADDRESS, AGENT_TOKEN.as_bytes(), NFT_ROLES);

        let validation_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(ValidationRegistryProxy)
            .init(identity_sc.clone())
            .code(VALIDATION_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(VALIDATION_SC_ADDRESS)
            .run();

        let reputation_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(ReputationRegistryProxy)
            .init(validation_sc.clone(), identity_sc.clone())
            .code(REPUTATION_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(REPUTATION_SC_ADDRESS)
            .run();

        // Deploy escrow with validation + identity addresses
        let escrow_sc = world
            .tx()
            .from(OWNER_ADDRESS)
            .typed(EscrowProxy)
            .init(validation_sc.clone(), identity_sc.clone())
            .code(ESCROW_CODE)
            .returns(ReturnsNewManagedAddress)
            .new_address(ESCROW_SC_ADDRESS)
            .run();

        // Set up accounts
        world.account(AGENT_OWNER).nonce(1).balance(1_000_000u64);
        world
            .account(CLIENT)
            .nonce(1)
            .balance(1_000_000u64)
            .esdt_balance(PAYMENT_TOKEN, 1_000_000_000u64)
            .esdt_balance(WRONG_TOKEN, 1_000_000_000u64);
        world.account(WORKER).nonce(1).balance(1_000_000u64);
        world.account(VALIDATOR).nonce(1).balance(1_000_000u64);
        world
            .account(EMPLOYER)
            .nonce(1)
            .balance(10_000_000_000u64)
            .esdt_balance(PAYMENT_TOKEN, 1_000_000_000u64);

        Self {
            world,
            identity_sc,
            validation_sc,
            reputation_sc,
            escrow_sc,
        }
    }

    // ── Agent registration (reuse pattern) ──

    pub fn register_agent(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        name: &[u8],
        uri: &[u8],
        pubkey: &[u8],
        metadata: Vec<(&[u8], &[u8])>,
        services: Vec<(u32, u64, &[u8], u64)>,
    ) {
        let mut args = ManagedArgBuffer::<StaticApi>::new();
        args.push_arg(ManagedBuffer::<StaticApi>::from(name));
        args.push_arg(ManagedBuffer::<StaticApi>::from(uri));
        args.push_arg(ManagedBuffer::<StaticApi>::from(pubkey));
        args.push_arg(metadata.len());
        for (k, v) in &metadata {
            args.push_arg(MetadataEntry::<StaticApi> {
                key: ManagedBuffer::from(*k),
                value: ManagedBuffer::from(*v),
            });
        }
        args.push_arg(services.len());
        for (sid, price, token, nonce) in &services {
            args.push_arg(ServiceConfigInput::<StaticApi> {
                service_id: *sid,
                price: BigUint::from(*price),
                token: TokenId::from(*token),
                nonce: *nonce,
            });
        }
        self.world
            .tx()
            .from(*from)
            .to(IDENTITY_SC_ADDRESS)
            .raw_call("register_agent")
            .arguments_raw(args)
            .run();
    }

    // ── Validation helpers ──

    pub fn init_job(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        agent_nonce: u64,
        service_id: Option<u32>,
    ) {
        let svc = match service_id {
            Some(sid) => OptionalValue::Some(sid),
            None => OptionalValue::None,
        };
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .init_job(ManagedBuffer::from(job_id), agent_nonce, svc)
            .run();
    }

    pub fn submit_proof(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        proof: &[u8],
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .submit_proof(ManagedBuffer::from(job_id), ManagedBuffer::from(proof))
            .run();
    }

    pub fn validation_request(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        validator: &multiversx_sc::types::TestAddress,
        request_uri: &[u8],
        request_hash: &[u8],
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .validation_request(
                ManagedBuffer::from(job_id),
                validator.to_managed_address(),
                ManagedBuffer::from(request_uri),
                ManagedBuffer::from(request_hash),
            )
            .run();
    }

    pub fn validation_response(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        request_hash: &[u8],
        response: u8,
        response_uri: &[u8],
        response_hash: &[u8],
        tag: &[u8],
    ) {
        self.world
            .tx()
            .from(*from)
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .validation_response(
                ManagedBuffer::from(request_hash),
                response,
                ManagedBuffer::from(response_uri),
                ManagedBuffer::from(response_hash),
                ManagedBuffer::from(tag),
            )
            .run();
    }

    // ── Escrow actions ──

    pub fn deposit_egld(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        receiver: &multiversx_sc::types::TestAddress,
        poa_hash: &[u8],
        deadline: u64,
        amount: u64,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(ESCROW_SC_ADDRESS)
            .typed(EscrowProxy)
            .deposit(
                ManagedBuffer::from(job_id),
                receiver.to_managed_address(),
                ManagedBuffer::from(poa_hash),
                deadline,
            )
            .egld(amount)
            .run();
    }

    pub fn deposit_egld_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        receiver: &multiversx_sc::types::TestAddress,
        poa_hash: &[u8],
        deadline: u64,
        amount: u64,
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(ESCROW_SC_ADDRESS)
            .typed(EscrowProxy)
            .deposit(
                ManagedBuffer::from(job_id),
                receiver.to_managed_address(),
                ManagedBuffer::from(poa_hash),
                deadline,
            )
            .egld(amount)
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn deposit_esdt(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        receiver: &multiversx_sc::types::TestAddress,
        poa_hash: &[u8],
        deadline: u64,
        token: &str,
        token_nonce: u64,
        amount: u64,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(ESCROW_SC_ADDRESS)
            .typed(EscrowProxy)
            .deposit(
                ManagedBuffer::from(job_id),
                receiver.to_managed_address(),
                ManagedBuffer::from(poa_hash),
                deadline,
            )
            .esdt(TestEsdtTransfer(
                multiversx_sc_scenario::imports::TestTokenIdentifier::new(token),
                token_nonce,
                amount,
            ))
            .run();
    }

    pub fn release(&mut self, from: &multiversx_sc::types::TestAddress, job_id: &[u8]) {
        self.world
            .tx()
            .from(*from)
            .to(ESCROW_SC_ADDRESS)
            .typed(EscrowProxy)
            .release(ManagedBuffer::from(job_id))
            .run();
    }

    pub fn release_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(ESCROW_SC_ADDRESS)
            .typed(EscrowProxy)
            .release(ManagedBuffer::from(job_id))
            .returns(ExpectMessage(err_msg))
            .run();
    }

    pub fn refund(&mut self, from: &multiversx_sc::types::TestAddress, job_id: &[u8]) {
        self.world
            .tx()
            .from(*from)
            .to(ESCROW_SC_ADDRESS)
            .typed(EscrowProxy)
            .refund(ManagedBuffer::from(job_id))
            .run();
    }

    pub fn refund_expect_err(
        &mut self,
        from: &multiversx_sc::types::TestAddress,
        job_id: &[u8],
        err_msg: &str,
    ) {
        self.world
            .tx()
            .from(*from)
            .to(ESCROW_SC_ADDRESS)
            .typed(EscrowProxy)
            .refund(ManagedBuffer::from(job_id))
            .returns(ExpectMessage(err_msg))
            .run();
    }

    // ── Escrow queries ──

    pub fn query_escrow(&mut self, job_id: &[u8]) -> EscrowData<StaticApi> {
        self.world
            .query()
            .to(ESCROW_SC_ADDRESS)
            .typed(EscrowProxy)
            .get_escrow(ManagedBuffer::from(job_id))
            .returns(ReturnsResult)
            .run()
    }

    pub fn query_is_job_verified(&mut self, job_id: &[u8]) -> bool {
        self.world
            .query()
            .to(VALIDATION_SC_ADDRESS)
            .typed(ValidationRegistryProxy)
            .is_job_verified(ManagedBuffer::from(job_id))
            .returns(ReturnsResult)
            .run()
    }

    /// Whitebox helper: directly set job status to Verified in the validation registry.
    ///
    /// NOTE: The current `validation_response` endpoint does NOT transition job status
    /// to `Verified` — it only updates `ValidationRequestData`. This is a known gap
    /// in the validation registry design. This helper simulates the expected behavior
    /// for escrow release testing.
    pub fn mark_job_verified(&mut self, job_id: &[u8]) {
        self.world
            .tx()
            .from(OWNER_ADDRESS)
            .to(VALIDATION_SC_ADDRESS)
            .whitebox(validation_registry::contract_obj, |sc| {
                let job_id_buf = ManagedBuffer::from(job_id);
                sc.job_data(&job_id_buf).update(|job| {
                    job.status = common::structs::JobStatus::Verified;
                });
            });
    }
}
