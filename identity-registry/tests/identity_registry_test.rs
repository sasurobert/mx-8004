use identity_registry::*;
use multiversx_sc_scenario::imports::OptionalValue;
use multiversx_sc_scenario::rust_biguint;
use multiversx_sc_scenario::testing_framework::BlockchainStateWrapper;

const WASM_PATH: &str = "output/identity-registry.wasm";

#[test]
fn test_register_agent() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        WASM_PATH,
    );

    b_mock
        .execute_tx(&owner_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.agent_token_id()
                .set_token_id(TokenIdentifier::from("AGENT-123456"));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        id_wrapper.address_ref(),
        b"AGENT-123456",
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftUpdateAttributes],
    );

    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("ipfs://hash"),
                ManagedBuffer::from("public_key_hex"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    let _ = b_mock.execute_query(&id_wrapper, |sc| {
        let agent = sc.get_agent(1);
        assert_eq!(agent.name, ManagedBuffer::from("Moltbot-01"));
        assert_eq!(sc.get_agent_id(ManagedAddress::from(agent_addr.clone())), 1);
    });
}

#[test]
fn test_update_agent_transfer_execute() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(1_000_000));

    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        WASM_PATH,
    );

    b_mock
        .execute_tx(&owner_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.agent_token_id()
                .set_token_id(TokenIdentifier::from("AGENT-123456"));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        id_wrapper.address_ref(),
        b"AGENT-123456",
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftUpdateAttributes],
    );

    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("PaidAgent"),
                ManagedBuffer::from("uri"),
                ManagedBuffer::from("pk"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    b_mock
        .execute_esdt_transfer(
            &agent_addr,
            &id_wrapper,
            b"AGENT-123456",
            1,
            &rust_biguint!(1),
            |sc| {
                let mut metadata = ManagedVec::new();
                metadata.push(MetadataEntry {
                    key: ManagedBuffer::from("price:chat"),
                    value: ManagedBuffer::from(b"\xC8"), // 200 in hex
                });

                sc.update_agent(
                    ManagedBuffer::from("new_uri"),
                    ManagedBuffer::from("new_pk"),
                    OptionalValue::Some(metadata),
                );
            },
        )
        .assert_ok();

    let _ = b_mock.execute_query(&id_wrapper, |sc| {
        let agent = sc.get_agent(1);
        assert_eq!(agent.uri, ManagedBuffer::from("new_uri"));
        let price = sc
            .agent_service_price(1, &ManagedBuffer::from("chat"))
            .get();
        assert_eq!(price, BigUint::from(200u32));
    });
}

#[test]
fn test_register_agent_with_custom_payment_token() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        WASM_PATH,
    );

    b_mock
        .execute_tx(&owner_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.agent_token_id()
                .set_token_id(TokenIdentifier::from("AGENT-123456"));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        id_wrapper.address_ref(),
        b"AGENT-123456",
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftUpdateAttributes],
    );

    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            let mut metadata = ManagedVec::new();
            metadata.push(MetadataEntry {
                key: ManagedBuffer::from("price:chat"),
                value: ManagedBuffer::from(&b"\x64"[..]), // 100 in hex
            });
            metadata.push(MetadataEntry {
                key: ManagedBuffer::from("token:chat"),
                value: ManagedBuffer::from(&b"USDC-123456"[..]),
            });
            metadata.push(MetadataEntry {
                key: ManagedBuffer::from("pnonce:chat"),
                value: ManagedBuffer::from(&10u64.to_be_bytes()[..]),
            });

            sc.register_agent(
                ManagedBuffer::from("AgentOne"),
                ManagedBuffer::from("https://uri1.com"),
                ManagedBuffer::from("pubkey1"),
                OptionalValue::Some(metadata),
            );
        })
        .assert_ok();

    let _ = b_mock.execute_query(&id_wrapper, |sc| {
        let price = sc
            .agent_service_price(1, &ManagedBuffer::from("chat"))
            .get();
        let token = sc
            .agent_service_payment_token(1, &ManagedBuffer::from("chat"))
            .get();
        let p_nonce = sc
            .agent_service_payment_nonce(1, &ManagedBuffer::from("chat"))
            .get();

        assert_eq!(price, BigUint::from(100u32));
        assert!(token.is_esdt());
        assert_eq!(token.unwrap_esdt(), TokenIdentifier::from("USDC-123456"));
        assert_eq!(p_nonce, 10u64);
    });
}

#[test]
fn test_register_agent_default_price_zero() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        WASM_PATH,
    );

    b_mock
        .execute_tx(&owner_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.agent_token_id()
                .set_token_id(TokenIdentifier::from("AGENT-123456"));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        id_wrapper.address_ref(),
        b"AGENT-123456",
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftUpdateAttributes],
    );

    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("FreeAgent"),
                ManagedBuffer::from("uri"),
                ManagedBuffer::from("pk"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    let _ = b_mock.execute_query(&id_wrapper, |sc| {
        let price = sc
            .agent_service_price(1, &ManagedBuffer::from("chat"))
            .get();
        assert_eq!(price, BigUint::from(0u32));
    });
}

#[test]
fn test_get_agent_service_config() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        WASM_PATH,
    );

    b_mock
        .execute_tx(&owner_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.agent_token_id()
                .set_token_id(TokenIdentifier::from("AGENT-123456"));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        id_wrapper.address_ref(),
        b"AGENT-123456",
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftUpdateAttributes],
    );

    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            let mut metadata = ManagedVec::new();
            metadata.push(MetadataEntry {
                key: ManagedBuffer::from("price:chat"),
                value: ManagedBuffer::from(&b"\x64"[..]), // 100
            });
            metadata.push(MetadataEntry {
                key: ManagedBuffer::from("token:chat"),
                value: ManagedBuffer::from(&b"USDC-123456"[..]),
            });
            metadata.push(MetadataEntry {
                key: ManagedBuffer::from("pnonce:chat"),
                value: ManagedBuffer::from(&10u64.to_be_bytes()[..]),
            });

            sc.register_agent(
                ManagedBuffer::from("AgentConfig"),
                ManagedBuffer::from("uri"),
                ManagedBuffer::from("pk"),
                OptionalValue::Some(metadata),
            );
        })
        .assert_ok();

    let _ = b_mock.execute_query(&id_wrapper, |sc| {
        let config = sc.get_agent_service_config(1, ManagedBuffer::from("chat"));

        assert_eq!(config.price, BigUint::from(100u32));
        assert!(config.token.is_esdt());
        assert_eq!(
            config.token.unwrap_esdt(),
            TokenIdentifier::from("USDC-123456")
        );
        assert_eq!(config.pnonce, 10u64);
    });
}

use multiversx_sc_scenario::*;

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();
    blockchain.register_contract(
        "file:../identity-registry/output/identity-registry.wasm",
        identity_registry::ContractBuilder,
    );
    blockchain
}

#[test]
fn identity_full_flow_scen() {
    world().run("../scenarios/identity_full_flow.scen.json");
}
