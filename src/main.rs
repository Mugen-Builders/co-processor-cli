mod commands;
mod helpers;
use crate::commands::create::create;
use crate::commands::devnet::{start_devnet, stop_devnet, update_devnet, reset_devnet};
use crate::helpers::helpers::{check_dependencies_installed, check_network_and_confirm_status};
use clap::{Parser, Subcommand};
use helpers::helpers::{
    address_book, check_deployment_environment, check_registration_environment,
};
use std::error::Error;

/// A CLI tool to interact with Web3.Storage
#[derive(Parser)]
#[command(author = "Idogwu Chinonso", version, about = "Bootstrap and deploy cartesi coprocesor programs easily from your CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(
        about = "Build and run all necessary steps to register and publish your program with co-processor"
    )]
    Publish {
        #[arg(short, long, help = "Your email address registered with Web3.Storage")]
        email: Option<String>,

        #[arg(
            short,
            long,
            help = "Environment where your program will be deployed to, e.g. Devnet, Mainnet or Testnet"
        )]
        network: String,
    },
    #[command(
        about = "Bootstrap a new directiry for your program",
        long_about = "Bootstrap a new directiry for your coprocessor program, this would contain both the cartesi template and also the solidity template"
    )]
    Create {
        #[arg(short, long, help = "Name of your program")]
        dapp_name: String,

        #[arg(short, long, help = "Language you intend to build with")]
        template: String,
    },

    #[command(
        about = "Start the devnet environment in detach mode",
        long_about = "Start the devnet environment in detach mode"
    )]
    StartDevnet,

    #[command(
        about = "Stop the devnet environment",
        long_about = "Stop the devnet environment"
    )]
    StopDevnet,

    #[command(
        about = "Check the coprocessor solver for status of the program download process",
        long_about = "Check the coprocessor solver for status of the program download process"
    )]
    PublishStatus {
        #[arg(
            short,
            long,
            help = "Environment where your program is registered to, e.g. Devnet, Mainnet or Testnet"
        )]
        network: String,
    },

    #[command(
        about = "Deploy the solidity code for your coprocessor program to any network of choice.",
        long_about = "Deploy the solidity code for your coprocessor program to any network of choice, by running the default deploy script (Deploy.s.sol)"
    )]
    Deploy {
        #[arg(short, long, help = "Name of your contract file")]
        contract_name: String,

        #[arg(
            short,
            long,
            help = "Environment where your program will be deployed to, e.g. Devnet, Mainnet or Testnet"
        )]
        network: String,

        #[arg(short, long, help = "Private key for deploying to selected network")]
        private_key: Option<String>,

        #[arg(short, long, help = "RPC for deploying to network of choice")]
        rpc: Option<String>,

        #[arg(
        short = 'a',
        long,
        help = "Constructor arguments to pass to the contract",
        num_args = 0..,
        value_delimiter = ' '
        )]
        constructor_args: Option<Vec<String>>,
    },

    #[command(about = "Pull the latest changes from the release branch for devnet")]
    UpdateDevnet,

    #[command(about = "Reset (delete & re-download) devnet")]
    ResetDevnet,

    #[command(
        about = "Displays the machine Hash and also co-processor address on different networks",
        long_about = "Displays the machine Hash and also co-processor address on different networks"
    )]
    AddressBook,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    match check_dependencies_installed() {
        false => Ok(()),
        true => match cli.command {
            Commands::Create {
                template,
                dapp_name,
            } => {
                create(dapp_name, template);
                Ok(())
            }
            Commands::StartDevnet => {
                start_devnet();
                Ok(())
            }
            Commands::StopDevnet => {
                stop_devnet();
                Ok(())
            }

            Commands::Publish { email, network } => {
                check_registration_environment(network, email);
                Ok(())
            }
            Commands::PublishStatus { network } => {
                check_network_and_confirm_status(network);
                Ok(())
            }

            Commands::Deploy {
                contract_name,
                network,
                private_key,
                rpc,
                constructor_args,
            } => {
                check_deployment_environment(
                    network,
                    private_key,
                    rpc,
                    constructor_args,
                    contract_name,
                );
                Ok(())
            }

            Commands::UpdateDevnet => {
                update_devnet();
                Ok(())
            }
            Commands::ResetDevnet => {
                reset_devnet();
                Ok(())
            }

            Commands::AddressBook => {
                address_book();
                Ok(())
            }
        },
    }
}
