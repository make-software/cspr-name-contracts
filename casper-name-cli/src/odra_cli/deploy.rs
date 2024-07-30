use casper_name_contracts::contracts::{
    controller::{Controller, ControllerInitArgs},
    name_token::{NameToken, NameTokenInitArgs},
    registrar::{Registrar, RegistrarInitArgs},
    resolver::{DefaultResolver, DefaultResolverInitArgs},
};
use odra::{host::Deployer, Addressable};

pub struct DeployScript;

impl odra_cli::DeployScript for DeployScript {
    fn deploy(
        &self,
        container: &mut odra_cli::DeployedContractsContainer,
        env: &odra::host::HostEnv,
    ) {
        let admin = env.get_account(0);
        env.set_gas(300_000_000_000);
        let token = NameToken::deploy(
            &env,
            NameTokenInitArgs {
                name: "005_CN".to_string(),
                symbol: "005_CN".to_string(),
            },
        );
        container.add_contract(&token);

        env.set_gas(200_000_000_000);
        let resolver = DefaultResolver::deploy(
            &env,
            DefaultResolverInitArgs {
                name_token: *token.address(),
            },
        );
        container.add_contract(&resolver);

        env.set_gas(150_000_000_000);
        let registrar = Registrar::deploy(
            &env,
            RegistrarInitArgs {
                name_token: *token.address(),
                default_resolver: *resolver.address(),
            },
        );
        container.add_contract(&registrar);

        env.set_gas(250_000_000_000);
        let controller = Controller::deploy(
            &env,
            ControllerInitArgs {
                registrar: *registrar.address(),
                treasury: admin,
                signer: env.public_key(&admin),
            },
        );
        container.add_contract(&controller);
    }
}
