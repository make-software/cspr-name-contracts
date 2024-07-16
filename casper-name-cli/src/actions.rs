use std::str::FromStr;

use casper_name_contracts::contracts::controller::{
    Controller, ControllerHostRef, ControllerInitArgs,
};
use casper_name_contracts::contracts::registrar::{
    Registrar, RegistrarHostRef, RegistrarInitArgs, CONTROLLER_ROLE,
};
use casper_name_contracts::contracts::resolver::{DefaultResolverHostRef, DefaultResolverInitArgs};
use casper_name_contracts::data_structures::{NameMintInfo, PaymentVoucher, TokenizationVoucher};
use odra::args::Maybe;
use odra::casper_types::bytesrepr::{Bytes, ToBytes};
use odra::contract_def::HasIdent;
use odra::host::HostRef;
use odra::host::Deployer;

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
    let admin = env.get_account(0);

    env.set_gas(300_000_000_000);
    let token = NameTokenHostRef::deploy(
        &env,
        NameTokenInitArgs {
            name: "006_CN".to_string(),
            symbol: "006_CN".to_string(),
        },
    );
    contracts.add_contract(&NameToken::ident(), token.address());

    env.set_gas(150_000_000_000);
    let registrar = RegistrarHostRef::deploy(
        &env,
        RegistrarInitArgs {
            name_token: *token.address(),
            // TODO: replace with a resolver address
            default_resolver: *token.address(),
        },
    );
    contracts.add_contract(&Registrar::ident(), registrar.address());

    env.set_gas(250_000_000_000);
    let controller = ControllerHostRef::deploy(
        &env,
        ControllerInitArgs {
            registrar: *registrar.address(),
            treasury: admin,
            signer: env.public_key(&admin),
        },
    );
    contracts.add_contract(&Controller::ident(), controller.address());

    env.set_gas(200_000_000_000);
    let resolver = DefaultResolverHostRef::deploy(&env, DefaultResolverInitArgs {
        name_token: *token.address(),
    });
    contracts.add_contract(&DefaultResolverHostRef::ident(), resolver.address());
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

    // Whitelist the controller in the registrar.
    env.set_gas(1_000_000_000);
    contracts
        .registrar
        .grant_role(&CONTROLLER_ROLE, contracts.controller.address());

    // Set the grace period.
    env.set_gas(1_000_000_000);
    contracts.registrar.set_grace_period(GRACE_PERIOD);

    // Set default resolver.
    env.set_gas(1_000_000_000);
    contracts
        .registrar
        .set_default_resolver(*contracts.resolver.address());
}

pub fn registrar_register(name: &str, buyer: &str) {
    let env = odra_casper_livenet_env::env();
    let mut contracts = DeployedContracts::load(&env);

    let owner = Address::from_str(buyer).unwrap();
    let now: u64 = chrono::Utc::now().timestamp_millis() as u64;
    let token_expiration = now + EXPIRATION;
    let voucher_expiration = now + ONE_DAY;
    let names = vec![NameMintInfo::new(name, owner, token_expiration)];
    let voucher = TokenizationVoucher::new(names, voucher_expiration);

    env.set_gas(10_000_000_000);
    contracts.registrar.register(voucher);
}

pub fn controller_buy(name: &str) {
    let env = odra_casper_livenet_env::env();
    let contracts = DeployedContracts::load(&env);
    let admin = env.get_account(0);

    let now: u64 = chrono::Utc::now().timestamp_millis() as u64;
    let token_expiration = now + EXPIRATION;
    let voucher_expiration = token_expiration;
    let amount = 1_000_000_000.into();
    let payment_id = "pay_001";
    let names = vec![NameMintInfo::new(
        name,
        env.get_account(0),
        voucher_expiration,
    )];

    let payment_voucher = PaymentVoucher::new(amount, &payment_id, admin, names, token_expiration);
    let bytes: Bytes = payment_voucher.to_bytes().unwrap().into();
    let signature = env.sign_message(&bytes, &admin);

    env.set_gas(50_000_000_000);
    contracts
        .controller
        .with_tokens(amount)
        .buy(payment_voucher, signature);
}
