use casper_name_contracts::contracts::{
    controller::Controller, name_token::NameToken, registrar::Registrar, resolver::DefaultResolver,
    reverse_resolver::ReverseResolver,
};
use deploy::DeployScript;
use odra_cli::OdraCli;
use scenario::{
    CalculateSignature, CalculateTokenHash, RegisterTokenScenario, SetConfigScript,
    UpgradeNameToken, UpgradeReverseResolver,
};

mod deploy;
mod scenario;

pub fn cli() {
    OdraCli::new()
        .about("Casper Name Service CLI")
        .deploy(DeployScript)
        .contract::<DefaultResolver>()
        .contract::<Controller>()
        .contract::<Registrar>()
        .contract::<NameToken>()
        .contract::<ReverseResolver>()
        .scenario(SetConfigScript)
        .scenario(RegisterTokenScenario)
        .scenario(CalculateTokenHash)
        .scenario(CalculateSignature)
        .scenario(UpgradeReverseResolver)
        .scenario(UpgradeNameToken)
        .build()
        .run();
}
