use multiversx_sc_scenario::imports::*;

use identity_registry::*;
use multiversx_sc_scenario::imports::StaticApi;

const OWNER_ADDRESS: TestAddress = TestAddress::new("owner");
const REGISTRY_ADDRESS: TestSCAddress = TestSCAddress::new("registry");
const CODE_PATH: MxscPath = MxscPath::new("output/identity-registry.mxsc.json");

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new().executor_config(ExecutorConfig::full_suite());

    blockchain.register_contract(CODE_PATH, identity_registry::ContractBuilder);
    blockchain
}

#[test]
fn identity_registry_blackbox() {
    let mut world = world();

    world.start_trace();

    world
        .account(OWNER_ADDRESS)
        .nonce(1)
        .balance(1_000_000_000_000_000_000u64); // 1 EGLD

    let new_address = world
        .tx()
        .from(OWNER_ADDRESS)
        .typed(identity_registry_proxy::IdentityRegistryProxy)
        .init()
        .code(CODE_PATH)
        .new_address(REGISTRY_ADDRESS)
        .returns(ReturnsNewAddress)
        .run();

    assert_eq!(new_address, REGISTRY_ADDRESS);

    // Issue token properly using typed call and chained payment
    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(REGISTRY_ADDRESS)
        .typed(identity_registry_proxy::IdentityRegistryProxy)
        .issue_token(
            ManagedBuffer::from(b"IdentityToken"),
            ManagedBuffer::from(b"IDT"),
        )
        .egld(50_000_000_000_000_000u64) // 0.05 EGLD
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(REGISTRY_ADDRESS)
        .typed(identity_registry_proxy::IdentityRegistryProxy)
        .register_agent(
            ManagedBuffer::from(b"test_agent"),
            ManagedBuffer::from(b"https://agent.com"),
            ManagedBuffer::from(b"public_key"),
            OptionalValue::<
                ManagedVec<
                    StaticApi,
                    identity_registry::identity_registry_proxy::MetadataEntry<StaticApi>,
                >,
            >::None,
        )
        .run();

    // Check if agent details are stored using get_agent view
    let agent_details = world
        .query()
        .to(REGISTRY_ADDRESS)
        .typed(identity_registry_proxy::IdentityRegistryProxy)
        .get_agent(1u64)
        .returns(ReturnsResultUnmanaged)
        .run();

    assert_eq!(agent_details.name, ManagedBuffer::from(b"test_agent"));
}
