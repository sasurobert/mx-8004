use identity_registry::{self, IdentityRegistry};
use multiversx_sc::types::{EsdtLocalRole, ManagedAddress, ManagedBuffer, TokenIdentifier};
use multiversx_sc_scenario::imports::OptionalValue;
use multiversx_sc_scenario::rust_biguint;
use multiversx_sc_scenario::testing_framework::BlockchainStateWrapper;
use validation_registry::{self, ValidationRegistry};

const ID_WASM_PATH: &str = "output/identity-registry.wasm";
const VAL_WASM_PATH: &str = "output/validation-registry.wasm";

#[test]
fn test_init_job_already_initialized() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let user_addr = b_mock.create_user_account(&rust_biguint!(0));

    // Deploy Validation Registry
    let val_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        validation_registry::contract_obj,
        VAL_WASM_PATH,
    );

    // Init Job 1
    b_mock
        .execute_tx(&user_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.init_job(ManagedBuffer::from("job_1"), 1u64);
        })
        .assert_ok();

    // Init Job 1 again -> Fail
    b_mock
        .execute_tx(&user_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.init_job(ManagedBuffer::from("job_1"), 1u64);
        })
        .assert_user_error("Job already initialized");
}

#[test]
fn test_init_job_insufficient_payment() {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_addr = b_mock.create_user_account(&rust_biguint!(0));
    let user_addr = b_mock.create_user_account(&rust_biguint!(0));
    let agent_addr = b_mock.create_user_account(&rust_biguint!(0));

    // 1. Deploy Identity Registry
    let id_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        identity_registry::contract_obj,
        ID_WASM_PATH,
    );

    // 2. Deploy Validation Registry
    let val_wrapper = b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&owner_addr),
        validation_registry::contract_obj,
        VAL_WASM_PATH,
    );

    // 3. Configure Validation Registry
    let id_addr = id_wrapper.address_ref().clone();
    b_mock
        .execute_tx(&owner_addr, &val_wrapper, &rust_biguint!(0), |sc| {
            sc.set_identity_registry_address(ManagedAddress::from(id_addr));
        })
        .assert_ok();

    // 4. Setup Agent with Price
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

    // Register Agent with Price: 100 EGLD (implicit default from test if we don't set it, wait)
    // Let's set a specific price in metadata "price:100"
    // To do this strictly via `register_agent`, we need to pass metadata.
    // However, `register_agent` arguments are `ManagedVec<MetadataEntry>`.
    // It's easier to register then update/set_metadata if needed, or just rely on default 0.
    // Wait, default is price:0. We want to test INSUFFICIENT payment.
    // So we must set a price > 0.

    // We can't easily construct MetadataEntry in test without full setup.
    // But `register_agent` adds "price:0" if empty.
    // Let's register, then "set_metadata" to "price:100".

    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            sc.register_agent(
                ManagedBuffer::from("Moltbot-01"),
                ManagedBuffer::from("uri"),
                ManagedBuffer::from("pk"),
                OptionalValue::None,
            );
        })
        .assert_ok();

    // Set Price to 100 via `set_metadata`
    // We need to construct `MetadataEntry`.
    // Or we can manually set storage.
    // Manually setting storage is cleaner for edge case test setup to avoid boilerplate of struct construction in test.
    // Storage key: agentServicePrice + nonce + service_id
    // But we want to test interaction.

    // Let's assume we can set metadata.
    // Actually, `register_agent` logic: if metadata empty, push default.
    // Using `sc` in closure gives us access to internal types!
    b_mock
        .execute_tx(&agent_addr, &id_wrapper, &rust_biguint!(0), |sc| {
            use identity_registry::MetadataEntry;
            let mut entries = multiversx_sc::types::ManagedVec::new();
            entries.push(MetadataEntry {
                key: ManagedBuffer::from(b"price:default"), // service_id="default"
                value: ManagedBuffer::new_from_bytes(&[0x64u8]),
            });
            sc.set_metadata(1u64, entries);
        })
        .assert_ok();

    // 5. User tries to init job with payment 50 < 100
    b_mock.set_egld_balance(&user_addr, &rust_biguint!(1000));

    b_mock
        .execute_tx(&user_addr, &val_wrapper, &rust_biguint!(50), |sc| {
            sc.init_job_with_payment(
                ManagedBuffer::from("job_2"),
                1u64,
                ManagedBuffer::from("default"),
            );
        })
        .assert_user_error("Insufficient payment");
}
