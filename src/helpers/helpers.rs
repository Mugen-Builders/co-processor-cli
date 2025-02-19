use crate::commands::deploy::deploy_contract;
use crate::commands::publish::{
    devnet_register, devnet_register_program_with_coprocessor, mainnet_register,
    register_program_with_coprocessor, testnet_register,
};
use colored::Colorize;
use enum_iterator::{all, Sequence};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::{env, string};
use std::{thread, time};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Sequence)]
enum DeploymentOptions {
    Devnet,
    Testnet,
    Mainnet,
}

impl DeploymentOptions {
    fn to_string(&self) -> String {
        match self {
            DeploymentOptions::Devnet => "Devnet".to_string(),
            DeploymentOptions::Testnet => "Testnet".to_string(),
            DeploymentOptions::Mainnet => "Mainnet".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UploadResponse {
    pub upload_id: String,
    pub presigned_url: String,
}

impl UploadResponse {
    pub fn new(upload_id: String, presigned_url: String) -> Self {
        UploadResponse {
            upload_id,
            presigned_url,
        }
    }
}

/// @notice Helper Function to check all the required dependencies are installed
/// @returns a boolean value indicating whether or not all dependencies are installed
pub fn check_dependencies_installed() -> bool {
    let required_tools = vec!["forge", "cartesi", "docker", "curl"];

    for tool in &required_tools {
        if !check_installed(tool.to_string()).unwrap_or(false) {
            eprintln!(
                "{} is not installed. Please install {} and try again.",
                tool.bright_red(),
                tool
            );
            return false;
        }
    }

    true
}

/// @notice Internal Function to ensure that a dependency exists
/// @param tool The name of the tool to check if installed
/// @returns a Result to tell of the operation was sucessful
pub fn check_installed(tool: String) -> Result<bool, String> {
    let output = Command::new("which")
        .arg(tool)
        .output()
        .map_err(|e| format!("Failed to execute 'which': {}", e))?;

    Ok(output.status.success())
}

/// @notice Function to read the contents of a file
/// @param path The path to file to be read from
/// @param var_name: The name of the file to be read from
pub fn read_file(path: &str, var_name: &str) -> String {
    if !Path::new(path).exists() {
        eprintln!(
            "{} {} file '{}' does not exist.",
            "Error::".red(),
            var_name,
            path
        );
        std::process::exit(1);
    }
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read {} file '{}'", var_name, path));
    content.trim().to_string()
}

/// @notice Function to get all available spaces
/// @return a verc of string representing all the available spaces
pub fn check_available_space() -> Vec<String> {
    let mut child = Command::new("w3")
        .arg("space")
        .arg("ls")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute 'w3 space creation'");

    let stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));

    let mut available_spaces: Vec<String> = Vec::new();

    // Collect lines from stdout first
    let lines: Vec<String> = stdout
        .lines()
        .filter_map(|line| line.ok()) // Filter out errors
        .collect();

    // Process collected lines to extract available spaces
    for line in lines {
        let response: Vec<&str> = line.split_ascii_whitespace().collect();
        available_spaces.push((response[response.len() - 1]).to_string());
    }

    // print the available spaces
    // for space in available_spaces.clone() {
    //     println!("Available space: {}", space);
    // }

    return available_spaces;
}

/// @notice Helper Function to check if a particulr email is loged in on w3 storage
/// @param email the email address to check if logged in
/// @returns a boolean value indicating whether or not the execution was successfull
pub fn check_if_logged_in(email: String) -> bool {
    let mut child = Command::new("w3")
        .arg("account")
        .arg("ls")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute 'w3 account ls'");

    let stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));
    let stderr = BufReader::new(child.stderr.take().expect("Failed to capture stderr"));

    let start = time::Instant::now();

    let mut logged_in: Vec<String> = Vec::new();
    let email_name: Vec<&str> = email.split("@").collect();

    // Collect lines from stdout first
    let lines: Vec<String> = stdout
        .lines()
        .filter_map(|line| line.ok()) // Filter out errors
        .collect();

    // Process collected lines to extract available spaces
    for line in lines {
        if line.starts_with("did:mailto:gmail.com:") {
            // println!("{} {}", "--W3STORAGE:: ".green(), line[21..].green());
            logged_in.push(line[21..].to_string());
        }
    }

    thread::spawn(move || {
        for line in stderr.lines() {
            if let Ok(line) = line {
                eprintln!("{} {}", "W3STORAGE::NOTE::".yellow(), line.yellow());
            } else if let Err(e) = line {
                eprintln!("{} {}", "W3STORAGE::ERROR::".red(), e);
            }
        }
    });

    while start.elapsed().as_secs() < 300 {
        if let Some(status) = child.try_wait().expect("Failed to check process status") {
            if status.success() {
                for name in logged_in.clone() {
                    if email_name[0].to_lowercase() == name.to_lowercase() {
                        return true;
                    }
                }
            } else {
                return false;
            }
        }

        // Poll every 5 seconds
        thread::sleep(time::Duration::from_secs(5));
    }

    // If timeout occurs
    child
        .kill()
        .expect("Failed to terminate the login process.");
    return false;
}

/// @notice Function to get the machine hash
pub fn get_machine_hash() -> String {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let image_hash = current_dir.join(".cartesi/image/hash");

    let output = Command::new("xxd")
        .arg("-p")
        .arg(image_hash)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute 'xxd' command")
        .stdout
        .expect("Failed to capture xxd output");

    let output = BufReader::new(output)
        .lines()
        .map(|line| line.unwrap_or_default())
        .collect::<Vec<_>>()
        .concat();

    return output.trim().to_string();
}

/// @notice Function to check that we have valid arguents for deployment
pub fn check_deploymet_args(
    network: &String,
    private_key: Option<String>,
    rpc: Option<String>,
) -> bool {
    let mut reject: bool = false;
    if private_key.is_none() {
        println!(
            "{} {}",
            "Please provide a private key for deploying to".red(),
            network.to_lowercase().red()
        );
        reject = true;
    }
    if rpc.is_none() {
        println!(
            "{} {}",
            "Please provide a RPC endpoint for deploying to".red(),
            network.to_lowercase().red()
        );
        reject = true;
    }
    return reject;
}

/// @notice This function determines the deployment environment (Devnet, Testnet, or Mainnet)
/// @notice based on the given network string. It then performs the appropriate registration
/// @notice action for the specified environment using the provided email.
/// @param `network` A `String` representing the network environment to check. It should be one of "devnet", "testnet", or "mainnet".
/// @param `email` A `String` containing the email address to be used for registration (only for mainnet).
/// @param `solver_env` A `String` containing the user specified deployment environment for testnet.
pub fn check_registration_environment(
    network: String,
    solver_env: String,
    email: Option<String>,
    build: bool,
) {
    let mut environment: Option<DeploymentOptions> = None;

    for option in all::<DeploymentOptions>().collect::<Vec<_>>() {
        if network.to_lowercase() == option.to_string().to_lowercase() {
            environment = Some(option);
        }
    }

    if environment.is_none() {
        println!(
            "{}",
            "Invalid network environment, please select either devnet, mainnet, or testnet".red()
        );
        return;
    }

    if let Some(deployment_env) = environment {
        match deployment_env {
            DeploymentOptions::Devnet => {
                devnet_register(build);
            }
            DeploymentOptions::Testnet => {
                testnet_register(solver_env, build);
            }
            DeploymentOptions::Mainnet => {
                if let Some(email) = email {
                    mainnet_register(email, build);
                } else {
                    println!("{}", "Please enter a valid email linked to your web3 storage profile, using the '--email' flag".red());
                }
            }
        }
    }
}

/// @notice This Function determines the deployment environment (Devnet, Testnet, or Mainnet) based on the given network string and deploys the contract using the provided parameters.
/// @param `network` A `String` representing the network environment to deploy to. It should be one of "devnet", "testnet", or "mainnet".
/// @param `private_key` An `Option<String>` containing the private key for deployment. Required for "mainnet" and "testnet".
/// @param `rpc` An `Option<String>` containing the RPC endpoint for deployment. Required for "mainnet" and "testnet".
/// @param `constructor_args` An `Option<Vec<String>>` containing the constructor arguments for the contract, if any.
/// @param `contract_name` A `String` representing the name of the contract to be deployed.

pub fn check_deployment_environment(
    network: String,
    private_key: Option<String>,
    rpc: Option<String>,
    constructor_args: Option<Vec<String>>,
    contract_name: String,
) {
    match network.to_lowercase().as_str() {
        "mainnet" => {
            if check_deploymet_args(&network, private_key.clone(), rpc.clone()) {
                return;
            } else {
                deploy_contract(
                    private_key.unwrap(),
                    rpc.unwrap(),
                    constructor_args,
                    contract_name,
                );
            }
        }
        "testnet" => {
            if check_deploymet_args(&network, private_key.clone(), rpc.clone()) {
                return;
            } else {
                deploy_contract(
                    private_key.unwrap(),
                    rpc.unwrap(),
                    constructor_args,
                    contract_name,
                );
            }
        }
        "devnet" => {
            deploy_contract(
                String::from("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
                String::from("http://127.0.0.1:8545"),
                constructor_args,
                contract_name,
            );
        }
        _ => {
            println!(
                "{}",
                "Invalid network environment, please select either, devnet, mainnet, or testnet"
                    .red()
            );
            return;
        }
    }
}

/// @notice This function Creates and returns a new spinner progress bar with a custom style.
/// @returns A `ProgressBar` instance configured as a spinner with the specified style and update interval.
pub fn get_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();

    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    // Start the spinner
    spinner.enable_steady_tick(Duration::from_millis(100));
    return spinner;
}

/// @notice This function checks for the existence of a machine hash file in the current directory and returns its content if available.
/// @returns `Option<String>` - Returns `Some(String)` containing the machine hash if the file exists,  otherwise returns `None`.
pub fn display_machine_hash() -> Option<String> {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let image_hash = current_dir.join(".cartesi/image/hash");
    if !image_hash.exists() {
        return None;
    } else {
        let hash = get_machine_hash();
        return Some(hash);
    }
}

/// @notice This function displays the address book for the current machine.
pub fn address_book() {
    let possible_machine_hash = display_machine_hash();
    let mut _machine_hash = String::new();

    match possible_machine_hash {
        Some(hash) => {
            _machine_hash = format!("0x{}", hash);
        }
        None => _machine_hash = String::from("--"),
    }

    let data = vec![
        ("Machine Hash", _machine_hash),
        (
            "Devnet_task_issuer",
            String::from("0x95401dc811bb5740090279Ba06cfA8fcF6113778"),
        ),
        (
            "Testnet_task_issuer",
            String::from("0xff35E413F5e22A9e1Cc02F92dcb78a5076c1aaf3"),
        ),
        (
            "payment_token",
            String::from("0xc5a5C42992dECbae36851359345FE25997F5C42d"),
        ),
    ];

    // Calculate the width of the first column
    let max_width = data.iter().map(|(name, _)| name.len()).max().unwrap_or(0);

    for (name, value) in data {
        println!("{:<width$}  {}", name, value, width = max_width);
    }
}

/// @notice This function check the network passed then calls the appropriate function to check the status of a the registration process.
/// @param network The network where the registration process is happening.
pub fn check_network_and_confirm_status(network: String, solver_env: String) {
    let mut environment: Option<DeploymentOptions> = None;

    for option in all::<DeploymentOptions>().collect::<Vec<_>>() {
        if network.to_lowercase() == option.to_string().to_lowercase() {
            environment = Some(option);
        }
    }

    if environment.is_none() {
        println!(
            "{}",
            "Invalid network environment, please select either devnet, mainnet, or testnet".red()
        );
        return;
    }

    if let Some(deployment_env) = environment {
        match deployment_env {
            DeploymentOptions::Devnet => {
                devnet_register_program_with_coprocessor(None, None);
            }
            DeploymentOptions::Testnet => {
                let solver_url = get_solver_url(&solver_env);
                register_program_with_coprocessor(solver_url);
            }

            DeploymentOptions::Mainnet => {
                register_program_with_coprocessor(String::from(
                    "https://cartesi-coprocessor-solver.fly.dev",
                ));
            }
        }
    }
}

/// @notice This function returns the appropriate solver URL based on the solver environment provided.
/// @param `solver_env` A `String` containing the user specified deployment environment for testnet.
pub fn get_solver_url(solver_env: &str) -> String {
    match solver_env {
        "dev" => "https://cartesi-coprocessor-solver-dev.fly.dev",
        "test" => "https://cartesi-coprocessor-solver-test.fly.dev",
        "prod" => "https://cartesi-coprocessor-solver-prod.fly.dev",
        _ => "https://cartesi-coprocessor-solver-prod.fly.dev",
    }
    .to_string()
}

pub fn decode_string_to_bool(str: String, arg_name: &str) -> Result<bool, ()> {
    match str.to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => {
            println!(
                "{} contains {}",
                arg_name.red(),
                "invalid boolean value, please provide either true or false".red()
            );
            Err(())
        }
    }
}
