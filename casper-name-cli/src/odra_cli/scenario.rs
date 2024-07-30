use blake2::{digest::VariableOutput, Blake2bVar};
use casper_name_contracts::{
    contracts::{
        controller::Controller,
        name_token::NameToken,
        registrar::{Registrar, CONTROLLER_ROLE},
        resolver::DefaultResolver,
    },
    data_structures::{NameMintInfo, PaymentVoucher, TokenizationVoucher},
};
use odra::{
    args::Maybe,
    casper_types::{
        bytesrepr::{Bytes, ToBytes},
        AsymmetricType, PublicKey,
    },
    Address, Addressable,
};
use std::collections::HashMap;
use std::io::Write;
use std::str::FromStr;

const ONE_DAY: u64 = 86400000;
const GRACE_PERIOD: u64 = ONE_DAY * 2;
const EXPIRATION: u64 = ONE_DAY * 365;

pub struct SetConfigScript;

impl odra_cli::ScenarioMetadata for SetConfigScript {
    const NAME: &'static str = "config";
    const DESCRIPTION: &'static str = "Sets dependencies between contracts";
}

impl odra_cli::Scenario for SetConfigScript {
    fn run(
        &self,
        container: odra_cli::DeployedContractsContainer,
        env: &odra::host::HostEnv,
        _args: HashMap<String, String>,
    ) {
        let resolver_address = *container
            .get_ref::<DefaultResolver>(env)
            .expect("Contract not found")
            .address();
        let controller_address = *container
            .get_ref::<Controller>(env)
            .expect("Contract not found")
            .address();

        let mut registrar = container
            .get_ref::<Registrar>(env)
            .expect("Contract not found");

        // Whitelist the registrar in the name token.
        env.set_gas(1_000_000_000);
        container
            .get_ref::<NameToken>(env)
            .expect("Contract not found")
            .set_variables(
                Maybe::Some(true),
                Maybe::Some(vec![*registrar.address()]),
                Maybe::None,
            );

        // Whitelist the controller in the registrar.
        env.set_gas(1_000_000_000);
        registrar.grant_role(&CONTROLLER_ROLE, &controller_address);

        // Set the grace period.
        env.set_gas(1_000_000_000);
        registrar.set_grace_period(GRACE_PERIOD);

        // Set default resolver.
        env.set_gas(1_000_000_000);
        registrar.set_default_resolver(resolver_address);
    }
}

pub struct RegisterTokenScenario;

impl odra_cli::ScenarioMetadata for RegisterTokenScenario {
    const NAME: &'static str = "register-token";
    const DESCRIPTION: &'static str = "Registers a token";
}

impl odra_cli::Scenario for RegisterTokenScenario {
    fn run(
        &self,
        container: odra_cli::DeployedContractsContainer,
        env: &odra::host::HostEnv,
        args: HashMap<String, String>,
    ) {
        let owner = args["buyer"]
            .parse::<Address>()
            .expect("Should be a valid address");
        let token_validity = to_mills(
            args.get("token_validity")
                .map(|v| v.parse::<u64>().ok())
                .flatten(),
        );
        let token_expiration = now() + token_validity.unwrap_or(EXPIRATION);
        let voucher_expiration = now() + ONE_DAY;
        let names = vec![NameMintInfo::new(&args["name"], owner, token_expiration)];
        let voucher = TokenizationVoucher::new(names, voucher_expiration);

        env.set_gas(10_000_000_000);
        container
            .get_ref::<Registrar>(env)
            .unwrap()
            .register(voucher);
    }

    fn args(&self) -> Vec<odra_cli::CommandArg> {
        vec![
            odra_cli::CommandArg::new("name", "Name to register", true),
            odra_cli::CommandArg::new("buyer", "Address of the buyer", true),
            odra_cli::CommandArg::new("token_validity", "Token validity in seconds", false),
        ]
    }
}

fn to_mills(seconds: Option<u64>) -> Option<u64> {
    seconds
        .map(|s| std::time::Duration::from_secs(s))
        .map(|d| d.as_millis() as u64)
}

pub struct CalculateTokenHash;

impl odra_cli::ScenarioMetadata for CalculateTokenHash {
    const NAME: &'static str = "token-hash";
    const DESCRIPTION: &'static str = "Calculates the hash of a token";
}

impl odra_cli::Scenario for CalculateTokenHash {
    fn run(
        &self,
        _container: odra_cli::DeployedContractsContainer,
        _env: &odra::host::HostEnv,
        args: HashMap<String, String>,
    ) {
        let token_name = &args["token_name"];
        prettycli::info(&blake2b(token_name));
    }

    fn args(&self) -> Vec<odra_cli::CommandArg> {
        vec![odra_cli::CommandArg::new(
            "token_name",
            "Name of the token",
            true,
        )]
    }
}

pub struct CalculateSignature;

impl odra_cli::ScenarioMetadata for CalculateSignature {
    const NAME: &'static str = "signature";
    const DESCRIPTION: &'static str = "Calculates the signature of a token";
}

impl odra_cli::Scenario for CalculateSignature {
    fn run(
        &self,
        _container: odra_cli::DeployedContractsContainer,
        env: &odra::host::HostEnv,
        args: HashMap<String, String>,
    ) {
        let admin = env.get_account(0);

        let name = NameMintInfo {
            label: args["voucher.names.label"].parse().unwrap(),
            owner: args["voucher.names.owner"].parse().unwrap(),
            token_expiration: args["voucher.names.token_expiration"].parse().unwrap(),
        };

        let voucher = PaymentVoucher::new(
            args["voucher.payment.amount"].parse().unwrap(),
            args["voucher.payment.payment_id"].as_str(),
            args["voucher.payment.buyer"].parse().unwrap(),
            vec![name],
            args["voucher.voucher_expiration"].parse().unwrap(),
        );

        let bytes: Bytes = voucher.to_bytes().unwrap().into();
        let signature = env.sign_message(&bytes, &admin);

        let msg = format!("Signature: {:?}", signature);
        prettycli::info(&msg);
    }

    fn args(&self) -> Vec<odra_cli::CommandArg> {
        vec![
            odra_cli::CommandArg::new("voucher.payment.buyer", "", true),
            odra_cli::CommandArg::new("voucher.payment.payment_id", "", true),
            odra_cli::CommandArg::new("voucher.payment.amount", "", true),
            odra_cli::CommandArg::new("voucher.names.label", "", true).list_element(),
            odra_cli::CommandArg::new("voucher.names.owner", "", true).list_element(),
            odra_cli::CommandArg::new("voucher.names.token_expiration", "", true).list_element(),
            odra_cli::CommandArg::new("voucher.voucher_expiration", "", true),
        ]
    }
}

fn now() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
}

fn blake2b<T: AsRef<[u8]>>(data: T) -> String {
    let mut result = [0u8; 32];
    let mut hasher = <Blake2bVar as VariableOutput>::new(32).expect("should create hasher");
    let _ = hasher.write(data.as_ref());
    hasher
        .finalize_variable(&mut result)
        .expect("should copy hash to the result array");
    hex::encode(result)
}

fn _parse_address(addr: &str) -> Address {
    let public_key = PublicKey::from_hex(addr.as_bytes());
    match public_key {
        Ok(public_key) => Address::from(public_key),
        Err(_) => Address::from_str(addr).expect("Invalid address"),
    }
}
