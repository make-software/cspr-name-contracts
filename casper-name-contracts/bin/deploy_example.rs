use std::str::FromStr;

use casper_name::name_token::*;
use odra::host::HostEnv;
use odra::{
    host::{Deployer, HostRef, HostRefLoader},
    Address,
};

fn main() {
    let env = odra_casper_livenet_env::env();

    // Deploy or load the Register contract.
    let mut contract = deploy_register(&env);
    // let mut contract = load_register(&env);

    // Become minter and operator.
    env.set_gas(1_000_000_000);
    contract.grant_role(&MINTER_ROLE, &env.caller());
    env.set_gas(1_000_000_000);
    contract.grant_role(&OPERATOR_ROLE, &env.caller());

    // // Mint new token.
    let kpob_addr = Address::from_str(
        "account-hash-88048a339fa20a4a17d746716aec4e1391be2ffdae4c60b1125320a0a07a3755",
    )
    .unwrap();
    env.set_gas(10_000_000_000);
    contract.mint(kpob_addr, String::from("test-label"), 1915246299000);

    env.set_gas(5_000_000_000);
    contract.renew(
        String::from("44b7dfe6596e4668313215e4f12ee9650d911a59087944b077d5c087212b89b1"),
        2015246299000,
    );

    env.set_gas(5_000_000_000);
    contract.admin_burn(String::from(
        "44b7dfe6596e4668313215e4f12ee9650d911a59087944b077d5c087212b89b1",
    ));

    env.set_gas(10_000_000_000);
    contract.mint(kpob_addr, String::from("test-label2"), 1915246299000);

    env.set_gas(10_000_000_000);
    contract.mint(kpob_addr, String::from("test-label3"), 2015246299000);
}

#[allow(dead_code)]
fn deploy_register(env: &HostEnv) -> RegisterHostRef {
    env.set_gas(450_000_000_000);
    let contract = RegisterHostRef::deploy(
        &env,
        RegisterInitArgs {
            name: String::from("CN_004_NAME"),
            symbol: String::from("CN_004_SYMBOL"),
        },
    );
    println!("Contract deployed at: {:?}", contract.address());
    contract
}

fn load_register(env: &HostEnv) -> RegisterHostRef {
    RegisterHostRef::load(
        &env,
        Address::from_str("hash-a33cf9cabc97ad785214ec5fc38ae08ab47577e83d54bb16554c1ee72a187bfb")
            .unwrap(),
    )
}
