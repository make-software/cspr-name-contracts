use casper_name_contracts::contracts::{
    controller::Controller, marketplace::SecondaryMarket, name_token::NameToken, registrar::Registrar, resolver::DefaultResolver
};
use deploy::DeployScript;
use odra_cli::OdraCli;
use scenario::{CalculateSignature, CalculateTokenHash, RegisterTokenScenario, SetConfigScript};

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
        .contract::<SecondaryMarket>()
        .scenario(SetConfigScript)
        .scenario(RegisterTokenScenario)
        .scenario(CalculateTokenHash)
        .scenario(CalculateSignature)
        .build()
        .run();
}
