use casper_name_contracts::contracts::{
    controller::{Controller, ControllerInitArgs},
    marketplace::{SecondaryMarket, SecondaryMarketInitArgs},
    name_token::{NameToken, NameTokenInitArgs},
    registrar::{Registrar, RegistrarInitArgs},
    resolver::{DefaultResolver, DefaultResolverInitArgs},
    reverse_resolver::{ReverseResolver, ReverseResolverInitArgs},
};
use odra::host::{Deployer, HostEnv};
use odra::prelude::*;

pub struct DeployScript;

impl odra_cli::deploy::DeployScript for DeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut odra_cli::DeployedContractsContainer,
    ) -> Result<(), odra_cli::deploy::Error> {
        let admin = env.get_account(0);
        env.set_gas(500_000_000_000);
        let token = NameToken::try_deploy(
            &env,
            NameTokenInitArgs {
                name: "008_CN".to_string(),
                symbol: "008_CN".to_string(),
                max_supply: 1_000_000,
            },
        )?;
        container.add_contract(&token)?;

        env.set_gas(300_000_000_000);
        let resolver = DefaultResolver::try_deploy(
            &env,
            DefaultResolverInitArgs {
                name_token: token.address(),
            },
        )?;
        container.add_contract(&resolver)?;

        env.set_gas(500_000_000_000);
        let registrar = Registrar::try_deploy(
            &env,
            RegistrarInitArgs {
                name_token: token.address(),
            },
        )?;
        container.add_contract(&registrar)?;

        env.set_gas(500_000_000_000);
        let controller = Controller::try_deploy(
            &env,
            ControllerInitArgs {
                registrar: registrar.address(),
                treasury: admin,
                signer: env.public_key(&admin),
            },
        )?;
        container.add_contract(&controller)?;

        env.set_gas(500_000_000_000);
        let market = SecondaryMarket::try_deploy(
            &env,
            SecondaryMarketInitArgs {
                signer: env.public_key(&admin),
                treasury: admin,
                name_token: token.address(),
            },
        )?;
        container.add_contract(&market)?;

        env.set_gas(300_000_000_000);
        let reverse_resolver = ReverseResolver::try_deploy(
            &env,
            ReverseResolverInitArgs {
                name_token: token.address(),
            },
        )?;
        container.add_contract(&reverse_resolver)?;

        Ok(())
    }
}
