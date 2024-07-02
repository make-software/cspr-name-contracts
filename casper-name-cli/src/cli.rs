use crate::actions;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "casper-name-cli")]
#[command(about = "Casper Name CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Deploys all DAO contracts
    DeployContracts,
    /// Configures whitelists of all contracts
    SetConfig,

    #[clap(name = "registrar", about = "Administer the registrar contract.")]
    Registrar(Registrar),
}

#[derive(Debug, Parser)]
pub struct Registrar {
    #[structopt(subcommand)]
    pub registrar_commands: RegistrarCommands,
}

#[derive(Debug, Subcommand)]
pub enum RegistrarCommands {
    Register { name: String, buyer: String },
}

pub fn parse() {
    match Cli::parse().command {
        Commands::DeployContracts => actions::deploy_all(),
        Commands::SetConfig => actions::set_config(),
        Commands::Registrar(registrar) => match registrar.registrar_commands {
            RegistrarCommands::Register { name, buyer } => {
                actions::registrar_register(&name, &buyer)
            }
        },
    }
}

fn _duration(seconds: Option<u64>) -> Option<std::time::Duration> {
    seconds.map(|s| std::time::Duration::from_secs(s))
}
