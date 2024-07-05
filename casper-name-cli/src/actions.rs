use std::str::FromStr;

use casper_name_contracts::contracts::registrar::{Registrar, RegistrarHostRef, RegistrarInitArgs};
use casper_name_contracts::data_structures::TokenizationVoucher;
use odra::args::Maybe;
use odra::contract_def::HasIdent;
use odra::host::Deployer;
use odra::host::HostRef;

use casper_name_contracts::contracts::name_token::{
    NameToken, NameTokenHostRef, NameTokenInitArgs,
};
use odra::Address;

use crate::deployed_contracts::{DeployedContracts, DeployedContractsToml};

pub const ONE_DAY: u64 = 86400000;
pub const GRACE_PERIOD: u64 = ONE_DAY * 2;
pub const EXPIRATION: u64 = ONE_DAY * 365;

pub fn deploy_all() {
    DeployedContractsToml::handle_previous_version();
    let mut contracts = DeployedContractsToml::new();
    let env = odra_casper_livenet_env::env();

    env.set_gas(300_000_000_000);
    let token = NameTokenHostRef::deploy(
        &env,
        NameTokenInitArgs {
            name: "004_CN".to_string(),
            symbol: "004_CN".to_string(),
        },
    );
    contracts.add_contract(&NameToken::ident(), token.address());

    env.set_gas(150_000_000_000);
    let registrar = RegistrarHostRef::deploy(
        &env,
        RegistrarInitArgs {
            name_token: token.address().clone(),
        },
    );
    contracts.add_contract(&Registrar::ident(), registrar.address());
}

pub fn set_config() {
    let env = odra_casper_livenet_env::env();
    let mut contracts = DeployedContracts::load(&env);

    // Whitelist the registrar in the name token.
    env.set_gas(1_000_000_000);
    contracts.token.set_variables(
        Maybe::Some(true),
        Maybe::Some(vec![contracts.registrar.address().clone()]),
        Maybe::None,
    );

    // Set the grace period.
    env.set_gas(1_000_000_000);
    contracts.registrar.set_grace_period(GRACE_PERIOD);
}

pub fn registrar_register(name: &str, buyer: &str) {
    let env = odra_casper_livenet_env::env();
    let mut contracts = DeployedContracts::load(&env);

    let owner = Address::from_str(buyer).unwrap();
    let now: u64 = chrono::Utc::now().timestamp_millis() as u64;
    let token_expiration = now + EXPIRATION;
    let voucher_expiration = now + ONE_DAY;
    let voucher = TokenizationVoucher::new(name, owner, token_expiration, voucher_expiration);

    env.set_gas(10_000_000_000);
    contracts.registrar.register(vec![voucher]);
}
