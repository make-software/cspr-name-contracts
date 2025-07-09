use casper_name_contracts::contracts::{
    controller::{Controller, ControllerInitArgs},
    marketplace::{SecondaryMarket, SecondaryMarketInitArgs},
    name_token::{NameToken, NameTokenInitArgs},
    registrar::{Registrar, RegistrarInitArgs},
    resolver::{DefaultResolver, DefaultResolverInitArgs},
    reverse_resolver::{ReverseResolver, ReverseResolverInitArgs},
};
use odra::host::HostEnv;
use odra::prelude::*;
use odra_cli::{deploy::Error, DeployedContractsContainer, DeployerExt};

pub struct DeployScript;

impl odra_cli::deploy::DeployScript for DeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer,
    ) -> Result<(), Error> {
        let admin = env.get_account(0);
        let token = NameToken::load_or_deploy(
            &env,
            NameTokenInitArgs {
                name: "008_CN".to_string(),
                symbol: "008_CN".to_string(),
                max_supply: 1_000_000,
            },
            container,
            500_000_000_000,
        )?;

        _ = DefaultResolver::load_or_deploy(
            &env,
            DefaultResolverInitArgs {
                name_token: token.address(),
            },
            container,
            300_000_000_000,
        )?;

        let registrar = Registrar::load_or_deploy(
            &env,
            RegistrarInitArgs {
                name_token: token.address(),
            },
            container,
            500_000_000_000,
        )?;

        _ = Controller::load_or_deploy(
            &env,
            ControllerInitArgs {
                registrar: registrar.address(),
                treasury: admin,
                signer: env.public_key(&admin),
            },
            container,
            500_000_000_000,
        )?;

        _ = SecondaryMarket::load_or_deploy(
            &env,
            SecondaryMarketInitArgs {
                signer: env.public_key(&admin),
                treasury: admin,
                name_token: token.address(),
            },
            container,
            500_000_000_000,
        )?;

        _ = ReverseResolver::load_or_deploy(
            &env,
            ReverseResolverInitArgs {
                name_token: token.address(),
            },
            container,
            300_000_000_000,
        )?;

        Ok(())
    }
}
