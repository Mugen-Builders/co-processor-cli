mod commands;
mod helpers;
use crate::commands::create::create;
use crate::commands::devnet::{reset_devnet, start_devnet, stop_devnet, update_devnet};
use crate::commands::publish::build_cartesi_machine_and_generate_car;
use crate::helpers::helpers::{check_dependencies_installed, check_network_and_confirm_status};
use clap::{Parser, Subcommand};
use helpers::helpers::{
    address_book, check_deployment_environment, check_registration_environment,
    decode_string_to_bool,
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
    #[command(about = "Build the Cartesi machine + generate the .car file (no uploading).")]
    Build,

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

        #[arg(
            long,
            default_value = "true",
            help = "Specify either 'true' or 'false' depending on if you'll want to build your program before running"
        )]
        build: String,

        #[arg(
            long,
            default_value = "prod",
            help = "Specify dev/test/prod for the solver environment"
        )]
        environment: String,

        #[arg(
            long,
            default_value = "false",
            help = "If 'true', check solver status after uploading"
        )]
        check_status: String,

        #[arg(
            long,
            help = "Optional custom solver URL to override the default solver URL"
        )]
        solver_url: Option<String>,
    },
    #[command(
        about = "Bootstrap a new directory for your program",
        long_about = "Bootstrap a new directory for your coprocessor program, this would contain both the cartesi template and also the solidity template"
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

            Commands::Build => {
                build_cartesi_machine_and_generate_car();
                Ok(())
            }

            Commands::Publish {
                email,
                network,
                build,
                environment,
                check_status,
                solver_url,
            } => {
                let check_status = decode_string_to_bool(check_status, "check_status");
                let build = decode_string_to_bool(build, "build");

                if check_status != Err(()) && build != Err(()) {
                    check_registration_environment(
                        network.clone(),
                        environment.clone(),
                        email,
                        build.unwrap(),
                        solver_url,
                    );
                    if check_status.unwrap() {
                        check_network_and_confirm_status(network, environment);
                    }
                }

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
