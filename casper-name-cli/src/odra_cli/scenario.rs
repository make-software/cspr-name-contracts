use blake2::{digest::VariableOutput, Blake2bVar};
use casper_name_contracts::{
    contracts::{
        controller::Controller,
        name_token::NameToken,
        registrar::{Registrar, CONTROLLER_ROLE},
        resolver::DefaultResolver,
        reverse_resolver::ReverseResolver,
    },
    data_structures::{NameMintInfo, PaymentVoucher, TokenizationVoucher},
};
use odra::{
    casper_types::{
        bytesrepr::{Bytes, ToBytes},
        U512,
    },
    contract_def::HasIdent,
    host::{Deployer, HostEnv, NoArgs, UpgradeConfig},
    prelude::*,
    schema::casper_contract_schema::NamedCLType,
};
use odra_cli::{
    cspr, scenario::{Args, Error as ScenarioError, Scenario, ScenarioMetadata}, CommandArg, ContractProvider, DeployedContractsContainer
};
use odra_modules::{access::DEFAULT_ADMIN_ROLE, dapp::DappMetadata};
use std::io::Write;

const ONE_DAY: u64 = 86400000;
const GRACE_PERIOD: u64 = ONE_DAY * 2;
const EXPIRATION: u64 = ONE_DAY * 365;

pub struct SetConfigScript;

impl ScenarioMetadata for SetConfigScript {
    const NAME: &'static str = "config";
    const DESCRIPTION: &'static str = "Sets dependencies between contracts";
}

impl Scenario for SetConfigScript {
    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        _args: Args,
    ) -> Result<(), ScenarioError> {
        env.set_captures_events(false);

        let resolver_address = container.contract_ref::<DefaultResolver>(env)?.address();
        let controller_address = container.contract_ref::<Controller>(env)?.address();
        // let marketplace_address = container.contract_ref::<SecondaryMarket>(env)?.address();

        let mut registrar = container.contract_ref::<Registrar>(env)?;
        let mut name_token = container.contract_ref::<NameToken>(env)?;
        let mut resolver = container.contract_ref::<DefaultResolver>(env)?;

        // Set default resolver.
        env.set_gas(20_000_000_000);
        name_token.whitelist(env.get_account(0));
        name_token.set_default_resolver(resolver_address);

        // // Whitelist the registrar in the name token.
        env.set_gas(20_000_000_000);
        name_token.whitelist(registrar.address());

        // Whitelist controllers in the registrar.
        env.set_gas(10_000_000_000);
        registrar.grant_role(&CONTROLLER_ROLE, &controller_address);
        // env.set_gas(10_000_000_000);
        // registrar.grant_role(&CONTROLLER_ROLE, &marketplace_address);

        // Set the grace period.
        env.set_gas(10_000_000_000);
        registrar.set_grace_period(GRACE_PERIOD);

        // Set the name token permissions in the resolver.
        env.set_gas(10_000_000_000);
        resolver.grant_role(&DEFAULT_ADMIN_ROLE, &name_token.address());

        Ok(())
    }
}

pub struct RegisterTokenScenario;

impl ScenarioMetadata for RegisterTokenScenario {
    const NAME: &'static str = "register-token";
    const DESCRIPTION: &'static str = "Registers a token";
}

impl Scenario for RegisterTokenScenario {
    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        args: Args,
    ) -> Result<(), ScenarioError> {
        let owner = args.get_single::<Address>("buyer")?;
        let token_validity = to_mills(args.get_single::<u64>("token_validity").ok());

        let token_expiration = now() + token_validity.unwrap_or(EXPIRATION);
        let voucher_expiration = now() + ONE_DAY;
        let names = vec![NameMintInfo::new(
            &args.get_single::<String>("name")?,
            owner,
            token_expiration,
            "",
        )];
        let voucher = TokenizationVoucher::new(names, voucher_expiration);

        env.set_gas(10_000_000_000);
        container
            .contract_ref::<Registrar>(env)?
            .controller_register(voucher);
        Ok(())
    }

    fn args(&self) -> Vec<CommandArg> {
        vec![
            CommandArg::new("name", "Name to register", NamedCLType::String).required(),
            CommandArg::new("buyer", "Address of the buyer", NamedCLType::Key).required(),
            CommandArg::new(
                "token_validity",
                "Token validity in seconds",
                NamedCLType::U64,
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

impl ScenarioMetadata for CalculateTokenHash {
    const NAME: &'static str = "token-hash";
    const DESCRIPTION: &'static str = "Calculates the hash of a token";
}

impl Scenario for CalculateTokenHash {
    fn run(
        &self,
        _env: &HostEnv,
        _container: &DeployedContractsContainer,
        args: Args,
    ) -> Result<(), ScenarioError> {
        let token_name = args.get_single::<String>("token_name")?;
        prettycli::info(&blake2b(token_name));
        Ok(())
    }

    fn args(&self) -> Vec<CommandArg> {
        vec![CommandArg::new("token_name", "Name of the token", NamedCLType::String).required()]
    }
}

pub struct CalculateSignature;

impl ScenarioMetadata for CalculateSignature {
    const NAME: &'static str = "signature";
    const DESCRIPTION: &'static str = "Calculates the signature of a token";
}

impl Scenario for CalculateSignature {
    fn run(
        &self,
        env: &HostEnv,
        _container: &DeployedContractsContainer,
        args: Args,
    ) -> Result<(), ScenarioError> {
        let admin = env.get_account(0);

        let name = NameMintInfo {
            label: args.get_single::<String>("voucher.names.label")?,
            owner: args.get_single::<Address>("voucher.names.owner")?,
            token_expiration: args.get_single::<u64>("voucher.names.token_expiration")?,
            asset_uri: args.get_single::<String>("voucher.names.asset_uri")?,
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

    fn args(&self) -> Vec<CommandArg> {
        vec![
            CommandArg::new("voucher.payment.buyer", "", NamedCLType::Key).required(),
            CommandArg::new("voucher.payment.payment_id", "", NamedCLType::String).required(),
            CommandArg::new("voucher.payment.amount", "", NamedCLType::U512).required(),
            CommandArg::new("voucher.names.label", "", NamedCLType::String).required(),
            CommandArg::new("voucher.names.owner", "", NamedCLType::Key).required(),
            CommandArg::new("voucher.names.token_expiration", "", NamedCLType::U64).required(),
            CommandArg::new("voucher.voucher_expiration", "", NamedCLType::U64).required(),
            CommandArg::new("voucher.names.asset_uri", "", NamedCLType::String).required(),
        ]
    }
}

pub struct UpgradeReverseResolver;

impl ScenarioMetadata for UpgradeReverseResolver {
    const NAME: &'static str = "upgrade-reverse-resolver";
    const DESCRIPTION: &'static str = "Upgrade the reverse resolver contract.";
}

impl Scenario for UpgradeReverseResolver {
    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        _args: Args,
    ) -> Result<(), ScenarioError> {
        let reverse_resolver_addr = container
            .address_by_name(&ReverseResolver::ident())
            .unwrap();

        env.set_gas(cspr!(400));
        let _result = ReverseResolver::try_upgrade_with_cfg(
            env,
            reverse_resolver_addr,
            NoArgs,
            UpgradeConfig {
                package_named_key: String::from("ReverseResolver_contract_package"),
                force_create_upgrade_group: true,
                allow_key_override: true,
            },
        );

        Ok(())
    }
}

pub struct UpgradeDefaultResolver;

impl ScenarioMetadata for UpgradeDefaultResolver {
    const NAME: &'static str = "upgrade-default-resolver";
    const DESCRIPTION: &'static str = "Upgrade the DefaultResolver contract.";
}

impl Scenario for UpgradeDefaultResolver {
    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        _args: Args,
    ) -> Result<(), ScenarioError> {
        let resolver_addr = container
            .address_by_name(&DefaultResolver::ident())
            .unwrap();

        env.set_gas(cspr!(400));
        DefaultResolver::try_upgrade_with_cfg(
            env,
            resolver_addr,
            NoArgs,
            UpgradeConfig {
                package_named_key: DefaultResolver::ident(),
                force_create_upgrade_group: true,
                allow_key_override: true,
            },
        )?;

        Ok(())
    }
}

pub struct RegisterDapp;

impl ScenarioMetadata for RegisterDapp {
    const NAME: &'static str = "register-dapp";
    const DESCRIPTION: &'static str =
        "Sets the dapp metadata on the NameToken and registers the other contracts as dapp members.";
}

impl Scenario for RegisterDapp {
    fn args(&self) -> Vec<CommandArg> {
        vec![
            CommandArg::new("name", "Name of the dapp", NamedCLType::String),
            CommandArg::new(
                "description",
                "Description of the dapp",
                NamedCLType::String,
            ),
            CommandArg::new(
                "website_url",
                "Website URL of the dapp",
                NamedCLType::String,
            ),
            CommandArg::new("icon_url", "Icon URL of the dapp", NamedCLType::String),
        ]
    }

    /// Idempotent: the metadata is set only when `name` is given, and a
    /// contract that already belongs to the dapp is skipped. Every member must
    /// run code that exposes `get_dapp_registry`, so run the upgrades first.
    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        args: Args,
    ) -> Result<(), ScenarioError> {
        env.set_captures_events(false);
        let mut name_token = container.contract_ref::<NameToken>(env)?;

        if let Ok(name) = args.get_single::<String>("name") {
            let metadata = DappMetadata {
                name,
                description: args.get_single::<String>("description").unwrap_or_default(),
                website_url: args.get_single::<String>("website_url").unwrap_or_default(),
                icon_url: args.get_single::<String>("icon_url").unwrap_or_default(),
            };
            env.set_gas(cspr!(10));
            name_token.try_set_dapp_metadata(metadata)?;
        }

        let members = [
            container.contract_ref::<DefaultResolver>(env)?.address(),
            container.contract_ref::<Registrar>(env)?.address(),
            container.contract_ref::<Controller>(env)?.address(),
            container.contract_ref::<ReverseResolver>(env)?.address(),
        ];
        for member in members {
            if name_token.is_dapp_contract(&member) {
                prettycli::info(&format!("{:?} is already a dapp member", member));
                continue;
            }
            env.set_gas(cspr!(20));
            name_token.try_add_dapp_contract(&member, false)?;
            prettycli::info(&format!("{:?} registered as a dapp member", member));
        }

        Ok(())
    }
}

pub struct UpgradeNameToken;

impl ScenarioMetadata for UpgradeNameToken {
    const NAME: &'static str = "upgrade-name-token";
    const DESCRIPTION: &'static str = "Upgrade the NameToken contract.";
}

impl Scenario for UpgradeNameToken {
    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        _args: Args,
    ) -> Result<(), ScenarioError> {
        let name_token_addr = container
            .address_by_name(&NameToken::ident())
            .unwrap();

        env.set_gas(cspr!(400));
        NameToken::try_upgrade_with_cfg(
            env,
            name_token_addr,
            NoArgs,
            UpgradeConfig {
                package_named_key: NameToken::ident(),
                force_create_upgrade_group: false,
                allow_key_override: true,
            },
        )?;

        Ok(())
    }
}

pub struct UpgradeRegistrar;

impl ScenarioMetadata for UpgradeRegistrar {
    const NAME: &'static str = "upgrade-registrar";
    const DESCRIPTION: &'static str = "Upgrade the Registrar contract.";
}

impl Scenario for UpgradeRegistrar {
    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        _args: Args,
    ) -> Result<(), ScenarioError> {
        let registrar_addr = container
            .address_by_name(&Registrar::ident())
            .unwrap();

        env.set_gas(cspr!(400));
        Registrar::try_upgrade_with_cfg(
            env,
            registrar_addr,
            NoArgs,
            UpgradeConfig {
                package_named_key: Registrar::ident(),
                force_create_upgrade_group: true,
                allow_key_override: true,
            },
        )?;

        Ok(())
    }
}

pub struct UpgradeController;

impl ScenarioMetadata for UpgradeController {
    const NAME: &'static str = "upgrade-controller";
    const DESCRIPTION: &'static str = "Upgrade the Controller contract.";
}

impl Scenario for UpgradeController {
    fn run(
        &self,
        env: &HostEnv,
        container: &DeployedContractsContainer,
        _args: Args,
    ) -> Result<(), ScenarioError> {
        let controller_addr = container
            .address_by_name(&Controller::ident())
            .unwrap();

        env.set_gas(cspr!(400));
        Controller::try_upgrade_with_cfg(
            env,
            controller_addr,
            NoArgs,
            UpgradeConfig {
                package_named_key: Controller::ident(),
                force_create_upgrade_group: true,
                allow_key_override: true,
            },
        )?;

        Ok(())
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
