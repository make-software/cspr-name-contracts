use casper_name_contracts::contracts::{
    controller::{Controller, ControllerInitArgs},
    name_token::{NameToken, NameTokenInitArgs},
    registrar::{Registrar, RegistrarInitArgs},
    resolver::{DefaultResolver, DefaultResolverInitArgs},
    reverse_resolver::{ReverseResolver, ReverseResolverInitArgs},
};
use odra::host::{Deployer, HostEnv, OdraConfig};
use odra::prelude::*;
use odra_cli::{deploy::Error, DeployedContractsContainer};

pub struct DeployScript;

impl odra_cli::deploy::DeployScript for DeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer,
    ) -> Result<(), Error> {
        let admin = env.get_account(0);

        env.set_gas(400_000_000_000);
        let token = NameToken::try_deploy_with_cfg(
            &env,
            NameTokenInitArgs {
                name: "CSPR.name".to_string(),
                symbol: "NAME".to_string(),
                max_supply: 1_000_000_000,
            },
            Cfg::new("NameToken"),
        )?;
        container.add_contract(&token)?;

        env.set_gas(300_000_000_000);
        let resolver = DefaultResolver::try_deploy_with_cfg(
            &env,
            DefaultResolverInitArgs {
                name_token: token.address(),
            },
            Cfg::new("DefaultResolver"),
        )?;
        container.add_contract(&resolver)?;

        env.set_gas(400_000_000_000);
        let registrar = Registrar::try_deploy_with_cfg(
            &env,
            RegistrarInitArgs {
                name_token: token.address(),
            },
            Cfg::new("Registrar"),
        )?;
        container.add_contract(&registrar)?;

        env.set_gas(400_000_000_000);
        let controller = Controller::try_deploy_with_cfg(
            &env,
            ControllerInitArgs {
                registrar: registrar.address(),
                treasury: admin,
                signer: env.public_key(&admin),
            },
            Cfg::new("Controller"),
        )?;
        container.add_contract(&controller)?;

        env.set_gas(300_000_000_000);
        let reverse_resolver = ReverseResolver::try_deploy_with_cfg(
            &env,
            ReverseResolverInitArgs {
                name_token: token.address(),
            },
            Cfg::new("ReverseResolver"),
        )?;
        container.add_contract(&reverse_resolver)?;

        Ok(())
    }
}

struct Cfg {
    name: &'static str,
}

impl Cfg {
    fn new(name: &'static str) -> Self {
        Cfg { name }
    }
}

impl OdraConfig for Cfg {
    fn package_hash(&self) -> String {
        String::from(self.name)
    }

    fn is_upgradable(&self) -> bool {
        true
    }

    fn allow_key_override(&self) -> bool {
        true
    }
}