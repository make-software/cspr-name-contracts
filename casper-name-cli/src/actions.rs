use blake2::digest::VariableOutput;
use blake2::Blake2bVar;
use casper_name_contracts::contracts::controller::{
    Controller, ControllerHostRef, ControllerInitArgs,
};
use casper_name_contracts::contracts::registrar::{
    Registrar, RegistrarHostRef, RegistrarInitArgs, CONTROLLER_ROLE,
};
use casper_name_contracts::contracts::resolver::{
    DefaultResolver, DefaultResolverHostRef, DefaultResolverInitArgs,
};
use casper_name_contracts::data_structures::{NameMintInfo, PaymentVoucher, TokenizationVoucher};
use odra::args::Maybe;
use odra::casper_types::bytesrepr::{Bytes, ToBytes};
use odra::casper_types::{AsymmetricType, PublicKey};
use odra::contract_def::HasIdent;
use odra::host::{Deployer, HostEnv, HostRef};
use std::io::Write;
use std::str::FromStr;

use casper_name_contracts::contracts::name_token::{
    NameToken, NameTokenHostRef, NameTokenInitArgs,
};
use odra::Address;

use crate::deployed_contracts::{DeployedContracts, DeployedContractsToml};

pub const ONE_DAY: u64 = 86400000;
pub const GRACE_PERIOD: u64 = ONE_DAY * 2;
pub const EXPIRATION: u64 = ONE_DAY * 365;

pub fn deploy_all(env: &HostEnv) {
    DeployedContractsToml::handle_previous_version();
    let mut contracts = DeployedContractsToml::new();
    let admin = env.get_account(0);

    env.set_gas(300_000_000_000);
    let token = NameTokenHostRef::deploy(
        &env,
        NameTokenInitArgs {
            name: "007_CN".to_string(),
            symbol: "007_CN".to_string(),
        },
    );
    contracts.add_contract(&NameToken::ident(), token.address());

    env.set_gas(200_000_000_000);
    let resolver = DefaultResolverHostRef::deploy(
        &env,
        DefaultResolverInitArgs {
            name_token: *token.address(),
        },
    );
    contracts.add_contract(&DefaultResolver::ident(), resolver.address());

    env.set_gas(150_000_000_000);
    let registrar = RegistrarHostRef::deploy(
        &env,
        RegistrarInitArgs {
            name_token: *token.address(),
            default_resolver: *resolver.address(),
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
}

pub fn set_config(env: &HostEnv) {
    let mut contracts = DeployedContracts::load(&env);

    // Whitelist the registrar in the name token.
    env.set_gas(1_000_000_000);
    contracts.token.set_variables(
        Maybe::Some(true),
        Maybe::Some(vec![*contracts.registrar.address()]),
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

pub mod registrar {
    use super::*;
    use casper_name_contracts::data_structures::{RenewalVoucher, TokenRenewalInfo};

    pub fn register(env: &HostEnv, name: String, buyer: String, token_validity: Option<u64>) {
        let owner = parse_address(&buyer);
        let token_expiration = now() + token_validity.unwrap_or(EXPIRATION);
        let voucher_expiration = now() + ONE_DAY;
        let names = vec![NameMintInfo::new(&name, owner, token_expiration)];
        let voucher = TokenizationVoucher::new(names, voucher_expiration);

        env.set_gas(10_000_000_000);
        contract(env).register(voucher);
    }

    pub fn prolong(env: &HostEnv, name: String, token_validity: Option<u64>) {
        let token_expiration = now() + token_validity.unwrap_or(EXPIRATION);
        let voucher_expiration = now() + ONE_DAY;
        let names = vec![TokenRenewalInfo::new(name, token_expiration)];
        let voucher = RenewalVoucher::new(names, voucher_expiration);

        env.set_gas(10_000_000_000);
        contract(env).prolong(voucher);
    }

    pub fn set_default_resolver(env: &HostEnv, address: String) {
        let resolver = parse_address(&address);
        assert!(resolver.is_contract());

        env.set_gas(3_000_000_000);
        contract(env).set_default_resolver(resolver);
    }

    pub fn set_grace_period(env: &HostEnv, seconds: u64) {
        env.set_gas(3_000_000_000);
        contract(env).set_grace_period(seconds);
    }

    pub fn resolve(env: &HostEnv, domain: String) {
        env.set_gas(10_000_000_000);
        contract(env).resolve(domain);
    }

    fn contract(env: &HostEnv) -> RegistrarHostRef {
        let contracts = DeployedContracts::load(env);
        contracts.registrar
    }
}

pub mod controller {
    use casper_name_contracts::data_structures::{RenewalPaymentVoucher, TokenRenewalInfo};

    use super::*;

    pub fn buy(env: &HostEnv, name: String, token_validity: Option<u64>) {
        let admin = env.get_account(0);

        let token_expiration = now() + token_validity.unwrap_or(EXPIRATION);
        let voucher_expiration = token_expiration;
        let amount = 1_000_000_000.into();
        let payment_id = "pay_001";
        let names = vec![NameMintInfo::new(&name, admin, voucher_expiration)];

        let payment_voucher =
            PaymentVoucher::new(amount, &payment_id, admin, names, token_expiration);
        let bytes: Bytes = payment_voucher.to_bytes().unwrap().into();
        let signature = env.sign_message(&bytes, &admin);

        env.set_gas(5_000_000_000);
        contract(env)
            .with_tokens(amount)
            .buy(payment_voucher, signature);
    }

    pub fn renew(env: &HostEnv, name: String, token_validity: Option<u64>) {
        let admin = env.get_account(0);

        let token_expiration = now() + token_validity.unwrap_or(EXPIRATION);
        let voucher_expiration = token_expiration;
        let amount = 1_000_000_000.into();
        let payment_id = "pay_001";
        let names = vec![TokenRenewalInfo::new(name, voucher_expiration)];

        let payment_voucher =
            RenewalPaymentVoucher::new(amount, &payment_id, admin, names, token_expiration);
        let bytes: Bytes = payment_voucher.to_bytes().unwrap().into();
        let signature = env.sign_message(&bytes, &admin);

        contract(env)
            .with_tokens(1_000_000_000.into())
            .renew(payment_voucher, signature)
    }

    pub fn resolve(env: &HostEnv, domain: String) {
        let result = contract(env).resolve(domain);

        let result = result
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "Not found".to_string());
        prettycli::info(&result);
    }

    fn contract(env: &HostEnv) -> ControllerHostRef {
        let contracts = DeployedContracts::load(env);
        contracts.controller
    }
}

pub mod token {
    use super::*;

    pub fn set_resolver(env: &HostEnv, token_name: String, address: String) {
        let resolver = parse_address(&address);
        assert!(resolver.is_contract());

        let token_hash = blake2b(token_name);
        env.set_gas(3_000_000_000);
        contract(env).set_resolver(token_hash, resolver);
    }

    pub fn resolver(env: &HostEnv, token_name: String) {
        let token_hash = blake2b(&token_name);
        let result = contract(env).resolver(token_hash);

        let result = result
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "Not found".to_string());

        let message = format!("Resolver for {:?}: {:?}", token_name, result);
        prettycli::info(&message);
    }

    pub fn metadata(env: &HostEnv, token_name: String) {
        let token_hash = blake2b(&token_name);
        let result = contract(env).try_metadata_by_hash(token_hash);

        let result = result
            .map(|metadata| serde_json::to_string_pretty(&metadata).ok())
            .ok()
            .flatten()
            .unwrap_or_else(|| "Not found".to_string());

        let message = format!("Metadata for {:?}:\n{}", token_name, result);
        prettycli::info(&message);
    }

    pub fn balance_of(env: &HostEnv, owner: String) {
        let owner = parse_address(&owner);
        env.set_gas(1_000_000_000);
        let result = contract(env).balance_of(owner);

        let message = format!("Balance of {:?}: {}", owner, result);
        prettycli::info(&message);
    }

    fn contract(env: &HostEnv) -> NameTokenHostRef {
        let contracts = DeployedContracts::load(env);
        contracts.token
    }
}

pub mod resolver {
    use super::*;

    pub fn set_resolution(env: &HostEnv, full_domain: String, address: String) {
        let addr = parse_address(&address);
        assert!(addr.is_contract());

        env.set_gas(10_000_000_000);
        contract(env).set_resolution(full_domain, Some(addr));
    }

    pub fn cleanup(env: &HostEnv, token_name: String) {
        env.set_gas(10_000_000_000);
        contract(env).cleanup(token_name);
    }

    pub fn resolve(env: &HostEnv, domain: String) {
        let result = contract(env).resolve(domain);

        let result = result
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "Not found".to_string());
        prettycli::info(&result);
    }

    fn contract(env: &HostEnv) -> DefaultResolverHostRef {
        let contracts = DeployedContracts::load(env);
        contracts.resolver
    }
}

pub(super) fn now() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
}

pub(super) fn blake2b<T: AsRef<[u8]>>(data: T) -> String {
    let mut result = [0u8; 32];
    let mut hasher = <Blake2bVar as VariableOutput>::new(32).expect("should create hasher");
    let _ = hasher.write(data.as_ref());
    hasher
        .finalize_variable(&mut result)
        .expect("should copy hash to the result array");
    hex::encode(result)
}

pub(super) fn parse_address(addr: &str) -> Address {
    let public_key = PublicKey::from_hex(addr.as_bytes());
    match public_key {
        Ok(public_key) => Address::from(public_key),
        Err(_) => Address::from_str(addr).expect("Invalid address"),
    }
}
