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

    #[clap(name = "controller", about = "Use the controller contract.")]
    Controller(Controller),

    #[clap(name = "token", about = "Use the token contract.")]
    NameToken(NameToken),

    #[clap(name = "resolver", about = "Use the resolver contract.")]
    Resolver(Resolver),
}

#[derive(Debug, Parser)]
pub struct Registrar {
    #[structopt(subcommand)]
    pub cmds: RegistrarCommands,
}

#[derive(Debug, Subcommand)]
pub enum RegistrarCommands {
    Register {
        name: String,
        buyer: String,
        token_validity_in_seconds: Option<u64>,
    },
    Prolong {
        name: String,
        token_validity_in_seconds: Option<u64>,
    },
    SetDefaultResolver {
        resolver: String,
    },
    Resolve {
        name: String,
    },
    SetGracePeriod {
        seconds: u64,
    },
}

#[derive(Debug, Parser)]
pub struct Controller {
    #[structopt(subcommand)]
    pub cmds: ControllerCommands,
}

#[derive(Debug, Subcommand)]
pub enum ControllerCommands {
    Buy {
        name: String,
        token_validity_in_seconds: Option<u64>,
    },
    Renew {
        name: String,
        token_validity_in_seconds: Option<u64>,
    },
    Resolve {
        domain: String,
    },
}

#[derive(Debug, Parser)]
pub struct NameToken {
    #[structopt(subcommand)]
    pub cmds: NameTokenCommands,
}

#[derive(Debug, Subcommand)]
pub enum NameTokenCommands {
    SetResolver {
        token_name: String,
        resolver: String,
    },
    Resolver {
        token_name: String,
    },
    Metadata {
        token_name: String,
    },
    BalanceOf {
        address: String,
    },
}

#[derive(Debug, Parser)]
pub struct Resolver {
    #[structopt(subcommand)]
    pub cmds: ResolverCommands,
}

#[derive(Debug, Subcommand)]
pub enum ResolverCommands {
    Cleanup {
        token_name: String,
    },
    SetResolution {
        full_domain: String,
        address: String,
    },
    Resolve {
        full_domain: String,
    },
}

pub fn parse() {
    let env = odra_casper_livenet_env::env();
    match Cli::parse().command {
        Commands::DeployContracts => actions::deploy_all(&env),
        Commands::SetConfig => actions::set_config(&env),
        Commands::Registrar(registrar) => match registrar.cmds {
            RegistrarCommands::Register {
                name,
                buyer,
                token_validity_in_seconds,
            } => {
                let token_validity = to_mills(token_validity_in_seconds);
                actions::registrar::register(&env, name, buyer, token_validity)
            }
            RegistrarCommands::Prolong {
                name,
                token_validity_in_seconds,
            } => {
                let token_validity = to_mills(token_validity_in_seconds);
                actions::registrar::prolong(&env, name, token_validity)
            }
            RegistrarCommands::SetDefaultResolver { resolver } => {
                actions::token::set_default_resolver(&env, resolver)
            }
            RegistrarCommands::Resolve { name } => actions::registrar::resolve(&env, name),
            RegistrarCommands::SetGracePeriod { seconds } => {
                actions::registrar::set_grace_period(&env, seconds)
            }
        },
        Commands::Controller(controller) => match controller.cmds {
            ControllerCommands::Buy {
                name,
                token_validity_in_seconds,
            } => {
                let token_validity = to_mills(token_validity_in_seconds);
                actions::controller::buy(&env, name, token_validity)
            }
            ControllerCommands::Renew {
                name,
                token_validity_in_seconds,
            } => {
                let token_validity = to_mills(token_validity_in_seconds);
                actions::controller::renew(&env, name, token_validity)
            }
            ControllerCommands::Resolve { domain } => actions::controller::resolve(&env, domain),
        },
        Commands::NameToken(token) => match token.cmds {
            NameTokenCommands::SetResolver {
                token_name,
                resolver,
            } => actions::token::set_resolver(&env, token_name, resolver),
            NameTokenCommands::Resolver { token_name } => {
                actions::token::resolver(&env, token_name)
            }
            NameTokenCommands::Metadata { token_name } => {
                actions::token::metadata(&env, token_name)
            }
            NameTokenCommands::BalanceOf { address } => actions::token::balance_of(&env, address),
        },
        Commands::Resolver(resolver) => match resolver.cmds {
            ResolverCommands::Cleanup { token_name } => {
                actions::resolver::cleanup(&env, token_name)
            }
            ResolverCommands::SetResolution {
                full_domain,
                address,
            } => actions::resolver::set_resolution(&env, full_domain, address),
            ResolverCommands::Resolve { full_domain } => {
                actions::resolver::resolve(&env, full_domain)
            }
        },
    }
}

fn to_mills(seconds: Option<u64>) -> Option<u64> {
    seconds
        .map(|s| std::time::Duration::from_secs(s))
        .map(|d| d.as_millis() as u64)
}
