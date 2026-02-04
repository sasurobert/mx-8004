use multiversx_sc_scenario::*;

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();
    blockchain.register_contract(
        "file:../identity-registry/output/identity-registry.wasm",
        identity_registry::contract_obj,
    );
    blockchain.register_contract(
        "file:../validation-registry/output/validation-registry.wasm",
        validation_registry::contract_obj,
    );
    blockchain.register_contract(
        "file:../reputation-registry/output/reputation-registry.wasm",
        reputation_registry::contract_obj,
    );
    blockchain
}

#[test]
fn authenticity_loop_scen() {
    world().run("scenarios/authenticity_loop.scen.json");
}

#[test]
fn identity_full_flow_scen() {
    world().run("scenarios/identity_full_flow.scen.json");
}
