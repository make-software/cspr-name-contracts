use blake2::{digest::VariableOutput, Blake2bVar};
use casper_name_contracts::{
    contracts::{
        controller::Controller,
        marketplace::SecondaryMarket,
        name_token::NameToken,
        registrar::{Registrar, CONTROLLER_ROLE},
        resolver::DefaultResolver,
    },
    data_structures::{NameMintInfo, PaymentVoucher, TokenizationVoucher},
};
use odra::{
    casper_types::{
        bytesrepr::{Bytes, ToBytes},
        AsymmetricType, PublicKey, U512,
    },
    prelude::*,
    schema::casper_contract_schema::NamedCLType,
    Addressable,
};
use std::io::Write;

const ONE_DAY: u64 = 86400000;
const GRACE_PERIOD: u64 = ONE_DAY * 2;
const EXPIRATION: u64 = ONE_DAY * 365;

pub struct SetConfigScript;

impl odra_cli::scenario::ScenarioMetadata for SetConfigScript {
    const NAME: &'static str = "config";
    const DESCRIPTION: &'static str = "Sets dependencies between contracts";
}

impl odra_cli::scenario::Scenario for SetConfigScript {
    fn run(
        &self,
        env: &odra::host::HostEnv,
        container: odra_cli::DeployedContractsContainer,
        _args: odra_cli::scenario::Args,
    ) -> Result<(), odra_cli::scenario::Error> {
        let resolver_address = *container.get_ref::<DefaultResolver>(env)?.address();
        let controller_address = *container.get_ref::<Controller>(env)?.address();
        let marketplace_address = *container.get_ref::<SecondaryMarket>(env)?.address();

        let mut registrar = container.get_ref::<Registrar>(env)?;
        let mut name_token = container.get_ref::<NameToken>(env)?;

        // Set default resolver.
        env.set_gas(20_000_000_000);
        name_token.whitelist(env.get_account(0));
        name_token.set_default_resolver(resolver_address);

        // Whitelist the registrar in the name token.
        env.set_gas(20_000_000_000);
        name_token.whitelist(*registrar.address());

        // Whitelist controllers in the registrar.
        env.set_gas(2_000_000_000);
        registrar.grant_role(&CONTROLLER_ROLE, &controller_address);
        env.set_gas(2_000_000_000);
        registrar.grant_role(&CONTROLLER_ROLE, &marketplace_address);

        // Set the grace period.
        env.set_gas(2_000_000_000);
        registrar.set_grace_period(GRACE_PERIOD);

        Ok(())
    }
}

pub struct RegisterTokenScenario;

impl odra_cli::scenario::ScenarioMetadata for RegisterTokenScenario {
    const NAME: &'static str = "register-token";
    const DESCRIPTION: &'static str = "Registers a token";
}

impl odra_cli::scenario::Scenario for RegisterTokenScenario {
    fn run(
        &self,
        env: &odra::host::HostEnv,
        container: odra_cli::DeployedContractsContainer,
        args: odra_cli::scenario::Args,
    ) -> Result<(), odra_cli::scenario::Error> {
        let owner = args.get_single::<Address>("buyer")?;
        let token_validity = to_mills(args.get_single::<u64>("token_validity").ok());

        let token_expiration = now() + token_validity.unwrap_or(EXPIRATION);
        let voucher_expiration = now() + ONE_DAY;
        let names = vec![NameMintInfo::new(
            &args.get_single::<String>("name")?,
            owner,
            token_expiration,
        )];
        let voucher = TokenizationVoucher::new(names, voucher_expiration);

        env.set_gas(10_000_000_000);
        container
            .get_ref::<Registrar>(env)
            .unwrap()
            .controller_register(voucher);
        Ok(())
    }

    fn args(&self) -> Vec<odra_cli::CommandArg> {
        vec![
            odra_cli::CommandArg::new("name", "Name to register", NamedCLType::String, true, false),
            odra_cli::CommandArg::new(
                "buyer",
                "Address of the buyer",
                NamedCLType::Key,
                true,
                false,
            ),
            odra_cli::CommandArg::new(
                "token_validity",
                "Token validity in seconds",
                NamedCLType::U64,
                false,
                false,
            ),
        ]
    }
}

fn to_mills(seconds: Option<u64>) -> Option<u64> {
    seconds
        .map(|s| std::time::Duration::from_secs(s))
        .map(|d| d.as_millis() as u64)
}

pub struct CalculateTokenHash;

impl odra_cli::scenario::ScenarioMetadata for CalculateTokenHash {
    const NAME: &'static str = "token-hash";
    const DESCRIPTION: &'static str = "Calculates the hash of a token";
}

impl odra_cli::scenario::Scenario for CalculateTokenHash {
    fn run(
        &self,
        _env: &odra::host::HostEnv,
        _container: odra_cli::DeployedContractsContainer,
        args: odra_cli::scenario::Args,
    ) -> Result<(), odra_cli::scenario::Error> {
        let token_name = args.get_single::<String>("token_name")?;
        prettycli::info(&blake2b(token_name));
        Ok(())
    }

    fn args(&self) -> Vec<odra_cli::CommandArg> {
        vec![odra_cli::CommandArg::new(
            "token_name",
            "Name of the token",
            NamedCLType::String,
            true,
            false,
        )]
    }
}

pub struct CalculateSignature;

impl odra_cli::scenario::ScenarioMetadata for CalculateSignature {
    const NAME: &'static str = "signature";
    const DESCRIPTION: &'static str = "Calculates the signature of a token";
}

impl odra_cli::scenario::Scenario for CalculateSignature {
    fn run(
        &self,
        env: &odra::host::HostEnv,
        _container: odra_cli::DeployedContractsContainer,
        args: odra_cli::scenario::Args,
    ) -> Result<(), odra_cli::scenario::Error> {
        let admin = env.get_account(0);

        let name = NameMintInfo {
            label: args.get_single::<String>("voucher.names.label")?,
            owner: args.get_single::<Address>("voucher.names.owner")?,
            token_expiration: args.get_single::<u64>("voucher.names.token_expiration")?,
        };

        let voucher = PaymentVoucher::new(
            args.get_single::<U512>("voucher.payment.amount")?,
            args.get_single::<String>("voucher.payment.payment_id")?
                .as_str(),
            args.get_single::<Address>("voucher.payment.buyer")?,
            vec![name],
            args.get_single::<u64>("voucher.voucher_expiration")?,
        );

        let bytes: Bytes = voucher.to_bytes().unwrap().into();
        let signature = env.sign_message(&bytes, &admin);

        let msg = format!("Signature: {:?}", hex::encode(signature));
        prettycli::info(&msg);
        Ok(())
    }

    fn args(&self) -> Vec<odra_cli::CommandArg> {
        vec![
            odra_cli::CommandArg::new("voucher.payment.buyer", "", NamedCLType::Key, true, false),
            odra_cli::CommandArg::new(
                "voucher.payment.payment_id",
                "",
                NamedCLType::String,
                true,
                false,
            ),
            odra_cli::CommandArg::new("voucher.payment.amount", "", NamedCLType::U512, true, false),
            odra_cli::CommandArg::new("voucher.names.label", "", NamedCLType::String, true, false),
            odra_cli::CommandArg::new("voucher.names.owner", "", NamedCLType::Key, true, false),
            odra_cli::CommandArg::new(
                "voucher.names.token_expiration",
                "",
                NamedCLType::U64,
                true,
                false,
            ),
            odra_cli::CommandArg::new(
                "voucher.voucher_expiration",
                "",
                NamedCLType::U64,
                true,
                false,
            ),
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
