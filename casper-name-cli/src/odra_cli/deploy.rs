use casper_name_contracts::contracts::{
    controller::{Controller, ControllerInitArgs},
    name_token::{NameToken, NameTokenInitArgs},
    registrar::{Registrar, RegistrarInitArgs},
    resolver::{DefaultResolver, DefaultResolverInitArgs},
    reverse_resolver::{ReverseResolver, ReverseResolverInitArgs},
};
use odra::host::{HostEnv, InstallConfig};
use odra::prelude::*;
use odra_cli::{cspr, deploy::Error, DeployedContractsContainer, DeployerExt};

pub struct DeployScript;

impl odra_cli::deploy::DeployScript for DeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer,
    ) -> Result<(), Error> {
        let admin = env.get_account(0);

        let token = NameToken::load_or_deploy_with_cfg(
            &env,
            NameTokenInitArgs {
                name: "CSPR.name".to_string(),
                symbol: "NAME".to_string(),
                max_supply: 1_000_000_000,
            },
            InstallConfig::upgradable::<NameToken>(),
            container,
            cspr!(400),
        )?;

        let _resolver = DefaultResolver::load_or_deploy_with_cfg(
            &env,
            DefaultResolverInitArgs {
                name_token: token.address(),
            },
            InstallConfig::upgradable::<DefaultResolver>(),
            container,
            cspr!(300),
        )?;

        let registrar = Registrar::load_or_deploy_with_cfg(
            &env,
            RegistrarInitArgs {
                name_token: token.address(),
            },
            InstallConfig::upgradable::<Registrar>(),
            container,
            cspr!(400),
        )?;

        let _controller = Controller::load_or_deploy_with_cfg(
            &env,
            ControllerInitArgs {
                registrar: registrar.address(),
                treasury: admin,
                signer: env.public_key(&admin),
            },
            InstallConfig::upgradable::<Controller>(),
            container,
            cspr!(300),
        )?;

        let _reverse_resolver = ReverseResolver::load_or_deploy_with_cfg(
            &env,
            ReverseResolverInitArgs {
                name_token: token.address(),
            },
            InstallConfig::upgradable::<ReverseResolver>(),
            container,
            cspr!(300),
        )?;

        Ok(())
    }
}
