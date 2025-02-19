use crate::helpers::helpers::{
    check_available_space, check_if_logged_in, display_machine_hash, get_machine_hash,
    get_solver_url, get_spinner, read_file, UploadResponse,
};
use colored::Colorize;
use indicatif::ProgressBar;
use reqwest::blocking::ClientBuilder;
use reqwest::blocking::{multipart, Client};
use reqwest::StatusCode;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::time::Duration;
use std::{thread, time};

/// @notice Function to set the space where uploaded car files will be saved to
/// @param space_name The name of the space of choice
fn set_active_space(space_name: String) {
    let mut child = Command::new("w3")
        .arg("space")
        .arg("use")
        .arg(space_name.clone())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute 'w3 space creation'");

    let stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));
    let stderr = BufReader::new(child.stderr.take().expect("Failed to capture stderr"));

    // Collect lines from stdout first
    let lines: Vec<Result<String, std::io::Error>> = stdout.lines().collect();

    // Process collected lines to extract available spaces
    for line in lines {
        if line.is_ok() {
            println!("Switched to space: {}", space_name);
            println!("Space ID: {}", line.unwrap());
            return;
        } else {
            eprintln!("Failed to switch to space: {}", space_name);
            return;
        }
    }

    thread::spawn(move || {
        for line in stderr.lines() {
            match line {
                Ok(output) => {
                    println!("{} {}", "WEB3STORAGE::".red(), output.red());
                }
                Err(e) => eprintln!("Error reading stdout: {}", e),
            }
        }
    });
}

/// @notice Function to create a new space for car files upload
/// @param space The name of the space of choice
fn create_space(space: String) {
    println!("Creating a new storage space: {}", space);
    let mut child = Command::new("w3")
        .arg("space")
        .arg("create")
        .arg(&space.clone())
        .arg("--no-recovery")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute 'w3 space create'");

    let stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));
    let stderr = BufReader::new(child.stderr.take().expect("Failed to capture stderr"));

    let start = time::Instant::now();

    // Handle initial output and extract keyphrase
    thread::spawn(move || {
        for line in stdout.lines() {
            if let Ok(line) = line {
                println!("{} {}", "W3STORAGE::OUTPUT:: ".green(), line.green());
            }
        }
        return;
    });

    thread::spawn(move || {
        for line in stderr.lines() {
            if let Ok(line) = line {
                eprintln!("{} {}", "W3STORAGE::NOTE::".yellow(), line.yellow());
                if line.contains("- Waiting for payment plan to be selected") {
                    println!("{}", "W3STORAGE::INSTRUCTION:: Login to your W3 storage dashboard and complete your payment plan selection".yellow());
                    break;
                }
            } else if let Err(e) = line {
                eprintln!("{} {}", "W3STORAGE::ERROR::".red(), e);
            }
        }
    });

    // Wait for email verification or timeout
    while start.elapsed().as_secs() < 300 {
        if let Some(status) = child.try_wait().expect("Failed to check process status") {
            if status.success() {
                println!("✅ Successfully created your space.");
                break;
            } else {
                eprintln!("space creation process timed out. Please select your payment plan within the specified timeout.");
                break;
            }
        }

        // Poll every 2 seconds
        thread::sleep(time::Duration::from_secs(2));
    }

    // If timeout occurs
    child
        .kill()
        .expect("Failed to terminate the space creation process.");
}

/// @notice Function to upload a car file to the active space
/// @param file_path The path to the car file of choice to be uploaded
/// @returns a boolean value indicating whether or not the execution was successful
fn upload_car_file(file_path: String) -> bool {
    // Create a spinner and set the message
    let spinner = get_spinner();
    spinner.set_message("Uploading CAR file...");

    let mut child = Command::new("w3")
        .arg("up")
        .arg("--car")
        .arg(file_path.clone())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute 'w3 up'");

    let stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));

    thread::spawn(move || {
        for line in stdout.lines() {
            match line {
                Ok(output) => {
                    println!("{} {}", "W3STORAGE::".green(), output.green());
                }
                Err(e) => eprintln!("Error reading stdout: {}", e),
            }
        }
    });

    // Wait for email verification or timeout
    let start = time::Instant::now();
    while start.elapsed().as_secs() < 30000 {
        if let Some(status) = child.try_wait().expect("Failed to check process status") {
            if status.success() {
                spinner.finish_and_clear();
                println!(
                    "✅ {}",
                    "Successfully uploaded file to Web3.Storage.".green()
                );
                return true;
            } else {
                spinner.finish_and_clear();
                eprintln!("{}", "upload process failed.".red());
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

/// @notice Function to build a Cartesi project before the registration process
/// @returns a boolean value indicating whether or not the execution was successful
fn build_program() -> bool {
    // Create a spinner and set the message
    let spinner = get_spinner();
    spinner.set_message("Building Cartesi Program...");

    let child = Command::new("cartesi")
        .arg("build")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute 'cartesi build' command")
        .wait_with_output()
        .expect("Failed to wait for forge command to finish");

    if child.status.success() {
        spinner.finish_and_clear();
        let stdout = String::from_utf8_lossy(&child.stdout);
        println!("{} {}", "CARTESI::".green(), stdout.green());
        println!("✅ {}", "Cartesi Program built successfully.".green());
        return true;
    } else {
        spinner.finish_and_clear();
        let stderr = String::from_utf8_lossy(&child.stderr);
        println!("{} {}", "CARTESI::".red(), stderr.red());
        eprintln!("{}", "build process failed.".red());
        return false;
    }
}

/// @notice Function to run the Carize command to generate car files
/// @returns a boolean value indicating whether or not the execution was successful
fn run_carize_container() -> bool {
    let current_dir = env::current_dir().expect("Failed to get current directory");

    let artifacts_dir = current_dir.join(".cartesi").join("artifacts");
    std::fs::create_dir_all(&artifacts_dir).expect("Failed to create .cartesi/artifacts folder");

    println!("{}", "Running Cartesi Container...".yellow());
    let mut child = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("-v")
        .arg(format!(
            "{}:/data",
            current_dir
                .join(".cartesi/image")
                .to_str()
                .expect("failed to convert path to string")
        ))
        .arg("-v")
        .arg(format!(
            "{}:/output",
            artifacts_dir
                .to_str()
                .expect("Failed to get current directory")
        ))
        .arg("ghcr.io/zippiehq/cartesi-carize:latest")
        .arg("/carize.sh")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute 'cartesi build' command");

    let stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));
    let stderr = BufReader::new(child.stderr.take().expect("Failed to capture stderr"));

    let start = time::Instant::now();

    thread::spawn(move || {
        for line in stdout.lines() {
            match line {
                Ok(output) => {
                    println!("{} {}", "CARIZE::".green(), output.green());
                }
                Err(e) => eprintln!("Error reading stdout: {}", e),
            }
        }
    });

    thread::spawn(move || {
        for line in stderr.lines() {
            match line {
                Ok(output) => {
                    println!("{} {}", "CARIZE::".green(), output.red());
                }
                Err(e) => eprintln!("Error reading stdout: {}", e),
            }
        }
    });

    // Wait for email verification or timeout
    while start.elapsed().as_secs() < 300 {
        if let Some(status) = child.try_wait().expect("Failed to check process status") {
            if status.success() {
                println!("✅ {}", "CARIZE generated successfully.".green());
                return true;
            } else {
                eprintln!("{}", "car file generation process failed.".red());
                return false;
            }
        }

        // Poll every 5 seconds
        thread::sleep(time::Duration::from_secs(5));
    }

    // If timeout occurs
    child
        .kill()
        .expect("Failed to terminate the generatioon process.");
    return false;
}

/// @notice Function to call the co-processor task manager to register the machine, hash, grogram cid etc.
pub fn register_program_with_coprocessor(base_url: String) {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let artifacts_dir = current_dir.join(".cartesi").join("artifacts");
    let output_cid = artifacts_dir.join("output.cid");
    let output_size = artifacts_dir.join("output.size");

    let cid = read_file(
        output_cid
            .to_str()
            .expect("error converting path to string"),
        "CID",
    );
    let size = read_file(
        output_size
            .to_str()
            .expect("error converting path to string"),
        "SIZE",
    );
    let machine_hash = get_machine_hash();

    let curl_status = Command::new("curl")
        .arg("-X")
        .arg("POST")
        .arg(format!(
            "{}/ensure/{}/{}/{}",
            base_url, cid, machine_hash, size
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute curl POST command")
        .wait_with_output()
        .expect("Failed to wait for curl command to finish");

    if curl_status.status.success() {
        println!(
            "✅ {} {} {}",
            "Successfully sent request to co-processor.".green(),
            "for program".green(),
            machine_hash.green()
        );
        let stdout = String::from_utf8_lossy(&curl_status.stdout);
        println!("✅ {} {}", "RESPONSE::".green(), stdout.green());
    } else {
        eprintln!("Failed to send POST request.");
        let stderr = String::from_utf8_lossy(&curl_status.stderr);
        eprintln!("Error: {}", stderr);
    }
}

/// @notice Function to login into web3 storage
/// @param email The email address whick is linked or to be linked to web3 storage
/// @returns a boolean value indicating whether or not the execution was successful
fn login(email: String) -> bool {
    let mut child = Command::new("w3")
        .arg("login")
        .arg(email)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute 'w3 login'");

    let stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));
    let stderr = BufReader::new(child.stderr.take().expect("Failed to capture stderr"));

    let start = time::Instant::now();

    // Handle output in separate threads
    thread::spawn(move || {
        for line in stdout.lines() {
            if let Ok(line) = line {
                println!("{} {}", "W3STORAGE:: ".green(), line.green());
            }
        }
    });

    thread::spawn(move || {
        for line in stderr.lines() {
            if let Ok(line) = line {
                eprintln!("{} {}", "W3STORAGE::NOTE::".yellow(), line.yellow());
            } else if let Err(e) = line {
                eprintln!("{} {}", "W3STORAGE::ERROR::".red(), e);
            }
        }
    });

    // Wait for email verification or timeout
    while start.elapsed().as_secs() < 300 {
        if let Some(status) = child.try_wait().expect("Failed to check process status") {
            if status.success() {
                println!("✅ Successfully logged in to Web3.Storage.");
                return true;
            } else {
                eprintln!("Login process timed out. Please verify the email within the specified timeout.");
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

/// @notice Function to check of web3storage space about to be created already exists
/// @param space The name of the web3 storage space to be created
/// @returns a boolean value indicating whether or not the execution was successful
fn check_and_create_space(space: String) -> bool {
    let available_spaces = check_available_space();

    let existing_space: Vec<String> = available_spaces
        .into_iter()
        .filter(|space_name| space_name.to_lowercase() == space.to_lowercase())
        .collect();
    // println!("{:?} match is:", existing_space);

    if existing_space.is_empty() {
        println!("Creating space {}", space);
        create_space(space.clone());

        let new_available_spaces = check_available_space();
        let new_space: Vec<String> = new_available_spaces
            .into_iter()
            .filter(|space_name| space_name.to_lowercase() == space.to_lowercase())
            .collect();

        if new_space.is_empty() {
            println!("{}", "ERROR creating space".red());
            return false;
        } else {
            set_active_space(new_space[0].clone());
            return true;
        }
    } else {
        set_active_space(existing_space[0].clone());
        return true;
    }
}

/// @notice Function that check if the file to be uploaded exists then if yes, it calls the upload functrion
/// @returns a boolean value indicating whether or not the execution was successful
fn check_and_upload() -> bool {
    let car_file_name = "output.car";
    // Get the current directory
    let current_dir = env::current_dir().expect("Failed to get current directory");

    let artifacts_dir = current_dir.join(".cartesi").join("artifacts");

    // Build the full path to the CAR file
    let car_file_path = artifacts_dir.join("output.car");

    // Check if the file exists
    if !car_file_path.exists() {
        eprintln!(
            "{} The CAR file '{}' was not found in the current directory '{}'.",
            "Error::".red(),
            car_file_name,
            current_dir.display()
        );
        return false;
    }

    // Convert the path to a string
    let car_file = car_file_path
        .to_str()
        .expect("Failed to convert path to string");

    // println!("{} CAR file found: {}", "INFO::".green(), car_file);
    match upload_car_file(car_file.to_string()) {
        true => return true,
        false => return false,
    };
}

/// @notice Entry point function to chain all the different functions required to register a new program on mainnet
/// @param email The email of your choice, to be linked if not already to web3 storage
pub fn mainnet_register(email: String, build: bool) {
    match check_if_logged_in(email.clone()) {
        true => {}
        false => {
            let _is_logged_in = login(email.clone());
        }
    };

    if build {
        if !build_program() {
            return;
        }
    }

    match run_carize_container() {
        true => match check_and_create_space("cartesi-coprocessor-programs".to_string()) {
            true => match check_and_upload() {
                true => {
                    register_program_with_coprocessor(String::from(
                        "https://cartesi-coprocessor-solver.fly.dev",
                    ));
                }
                false => return,
            },
            false => return,
        },
        false => {
            return;
        }
    }
}

/// @notice Entry point function to chain all the different functions required to register a new program on mainnet
/// @param `solver_env` A `String` containing the user specified deployment environment for testnet.
/// @param `build` A `bool` specified by the user to configure if the program should be built again befor publishing.
pub fn testnet_register(solver_env: String, build: bool) {
    let solver_url = get_solver_url(&solver_env);

    if build {
        if !build_program() {
            return;
        }
    }

    match run_carize_container() {
        true => match get_pre_signed_url(solver_url.to_string()) {
            Some(_response) => return,
            None => return,
        },
        false => return,
    }
}

/// @notice Entry point function to chain all the different functions required to register a new program in devnet mode.
/// @param `build` A `bool` specified by the user to configure if the program should be built again befor publishing.
pub fn devnet_register(build: bool) {
    if build {
        if !build_program() {
            return;
        }
    }

    match run_carize_container() {
        true => match devnet_upload_car_file() {
            true => return,
            false => return,
        },
        false => {
            return;
        }
    }
}

/// @notice Function to call the co-processor task manager to register the machine, hash, grogram cid etc on Devnet.
/// @param spinner This is an instance of a spinner displayed to users to signify the running process.
pub fn devnet_register_program_with_coprocessor(spinner: Option<ProgressBar>, retries: Option<u8>) {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let output_cid = current_dir.join("output.cid");
    let output_size = current_dir.join("output.size");

    let cid = read_file(
        output_cid
            .to_str()
            .expect("error converting path to string"),
        "CID",
    );
    let size = read_file(
        output_size
            .to_str()
            .expect("error converting path to string"),
        "SIZE",
    );
    let machine_hash = get_machine_hash();

    let curl_status = Command::new("curl")
        .arg("-X")
        .arg("POST")
        .arg(format!(
            "http://127.0.0.1:3034/ensure/{}/{}/{}",
            cid, machine_hash, size
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute curl POST command")
        .wait_with_output()
        .expect("Failed to wait for curl command to finish");

    if curl_status.status.success() {
        let stdout = String::from_utf8_lossy(&curl_status.stdout);
        check_and_recal_devnet_solver_register(stdout.to_string(), spinner, retries);
    } else {
        eprintln!("Failed to send POST request.");
        let stderr = String::from_utf8_lossy(&curl_status.stderr);
        if stderr.contains("Failed to connect to") || stderr.contains("Couldn't connect to server")
        {
            println!(
                "{}",
                "Devnet container not running, run the stop and start devnet command!!".red()
            )
        } else {
            eprintln!("Error: {}", stderr);
        }
    }
}

/// @notice Function to get a presigned url where car files are to be uploaded from the solver
/// @param solver_url this is the url link of the solver on devnet
/// @return an Option containing either the presigned url or None
fn get_pre_signed_url(solver_url: String) -> Option<UploadResponse> {
    let client = Client::new();
    let res = client
        .post(format!("{}/upload", solver_url))
        .body("")
        .send()
        .expect("Failed to send request");

    match res.status() {
        StatusCode::OK => {
            let json_body: serde_json::Value = res.json().expect("Failed to parse JSON response");
            let upload_id = json_body
                .get("upload_id")
                .expect("Failed to parse upload ID")
                .to_string();
            let presigned_url = json_body
                .get("presigned_url")
                .expect("Failed to parse presigned URL")
                .to_string();
            let response = UploadResponse::new(upload_id.clone(), presigned_url.clone());
            upload_to_presigned_url(response.clone(), solver_url.clone());
            return Some(response);
        }
        _ => {
            eprintln!(
                "❌ {}",
                "ERROR:: Failed to receive presigned url from solver".red()
            );
            return None;
        }
    }
}

/// @notice Function to upload the car file to the presigned url
/// @param response A struct containing the presigned url as well as an upload Id.
/// @param solver_url this is the url link of the solver on devnet
/// @return true if successful else false
fn upload_to_presigned_url(response: UploadResponse, solver_url: String) -> bool {
    let car_file_name = "output.car";
    // Get the current directory
    let current_dir = env::current_dir().expect("Failed to get current directory");

    let artifacts_dir = current_dir.join(".cartesi").join("artifacts");

    // Build the full path to the CAR file
    let car_file_path = artifacts_dir.join("output.car");

    // Check if the file exists
    if !car_file_path.exists() {
        eprintln!(
            "{} The CAR file '{}' was not found in the current directory '{}'.",
            "Error::".red(),
            car_file_name,
            current_dir.display()
        );
        return false;
    }

    // Convert the path to a string
    let car_file = car_file_path
        .to_str()
        .expect("Failed to convert path to string");

    // Open the file
    let mut file = File::open(&car_file).expect("Failed to open file");

    // Read the file content into a buffer
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let spinner = get_spinner();
    spinner.set_message("Uploading CAR file...");

    let url = response.presigned_url[1..response.presigned_url.len() - 1].to_string();
    let upload_id = response.upload_id[1..response.upload_id.len() - 1].to_string();

    // Configure HTTP client with no timeout
    let client = Client::builder()
        .timeout(None)
        .build()
        .expect("Failed to build HTTP client");

    let res = client
        .put(&url)
        .body(buffer)
        .send()
        .expect("Failed to send PUT request to presigned URL");

    // Handle response
    match res.status() {
        StatusCode::OK | StatusCode::CREATED => {
            spinner.finish_and_clear();
            println!("✅ {}", "File uploaded successfully!".green());
            publish_upload_id(upload_id, solver_url);
            return true;
        }
        _ => {
            spinner.finish_and_clear();
            eprintln!(
                "❌ Upload failed. Status: {}, Error: {}",
                res.status(),
                res.text().unwrap_or_else(|_| "Unknown error".to_string())
            );
            return false;
        }
    }
}

/// @notice Function to call the solver and publish the upload Id so it can start downloading the previously uploaded car file
/// It has a set timeout of 300 seconds
/// @param solver_url this is the url link of the solver on devnet
/// @param upload_id this is the Id of the upload gotten from the get_pre_signed_url function
fn publish_upload_id(upload_id: String, solver_url: String) {
    let spinner = get_spinner();
    spinner.set_message("Publishing upload Id...");

    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(300))
        .build()
        .expect("Failed to build Reqwest client");

    let response = client
        .post(format!("{}/publish/{}", solver_url.clone(), upload_id))
        .body("")
        .send();

    match response {
        Ok(res) if res.status().is_success() => {
            spinner.finish_and_clear();
            println!("✅ {}", "Upload ID published successfully!".green());
            check_publish_status(upload_id, None, None, solver_url);
        }
        Ok(_) | Err(_) => {
            spinner.finish_and_clear();
            eprintln!("❌ {}", "Failed to publish upload ID:".red());
        }
    }
}

/// @notice Function to check the status of the upload_id publish process it contains a loop to recheck the status 5 times
/// before finally breaking if it does not receive a possitive response
/// @param solver_url this is the url link of the solver on devnet
/// @param upload_id this is the Id of the upload gotten from the get_pre_signed_url function
/// @param retries this tracks the number of times the recursion has happened so we dont exceed 5
/// @param spinner This is an instance of a spinner displayed to users to signify the running process.
fn check_publish_status(
    upload_id: String,
    spinner: Option<ProgressBar>,
    retries: Option<u8>,
    solver_url: String,
) {
    let client = Client::new();
    let response = client
        .get(format!(
            "{}/publish_status/{}",
            solver_url.clone(),
            upload_id
        ))
        .send()
        .expect("errorchecking status");

    if !response.status().is_success() {
        eprintln!("❌ {}", "Failed to check status: ".red());
    } else {
        let json_body: serde_json::Value = response.json().expect("Failed to parse JSON response");

        let result = json_body["publish_results"]
            .as_array()
            .expect("Failed to parse JSON response")[0]
            .clone();

        let result_obj = serde_json::Value::from(result);
        let response_body = result_obj["response_body"]
            .as_str()
            .expect("failed to get state");

        if response_body.contains("upload_failed") | response_body.contains("dag_import_error") {
            println!("❌ {}", "Publish failed, please check the logs".red());
            return;
        } else if response_body.contains("dag_importing_complete") {
            if let Some(new_spinner) = spinner {
                new_spinner.finish_and_clear();
            }
            println!("✅ {}", "DAG imported successfully!".green());
            register_program_with_coprocessor(solver_url.clone());
            return;
        } else {
            std::thread::sleep(time::Duration::from_secs(5));

            let mut retries_count: u8 = 0;

            if let Some(retries) = retries {
                if retries >= 5 {
                    println!(
                        "❌ {} {} {}",
                        "Solver failed to finish setup after ".red(),
                        retries,
                        "retries".red()
                    );
                    return;
                }
                retries_count = retries;
            }

            if let Some(mut new_spinner) = spinner {
                new_spinner.finish_and_clear();
                new_spinner = get_spinner();
                new_spinner.set_message("Waiting for solver to finish publication process...");

                check_publish_status(
                    upload_id,
                    Some(new_spinner),
                    Some(retries_count + 1),
                    solver_url.clone(),
                );
            } else {
                let new_spinner = get_spinner();
                new_spinner.set_message("Waiting for solver to finish publication process...");
                check_publish_status(
                    upload_id,
                    Some(new_spinner),
                    Some(retries_count + 1),
                    solver_url.clone(),
                );
            }
        }
    }
}

/// @notice Function to call the import endpoint of the co-processor solver in devnet mode.
/// @return boolean to symbolise the status of the process.
fn devnet_upload_car_file() -> bool {
    let car_file_name = "output.car";
    // Get the current directory
    let current_dir = env::current_dir().expect("Failed to get current directory");

    let artifacts_dir = current_dir.join(".cartesi").join("artifacts");

    // Build the full path to the CAR file
    let car_file_path = artifacts_dir.join("output.car");

    // Check if the file exists
    if !car_file_path.exists() {
        eprintln!(
            "{} The CAR file '{}' was not found in the current directory '{}'.",
            "Error::".red(),
            car_file_name,
            current_dir.display()
        );
        return false;
    }

    // Convert the path to a string
    let car_file = car_file_path
        .to_str()
        .expect("Failed to convert path to string");

    let url = "http://127.0.0.1:5001/api/v0/dag/import";

    let form = multipart::Form::new()
        .file("file", car_file)
        .expect("unable to create form");

    let spinner = get_spinner();
    spinner.set_message("Uploading CAR file...");

    let client = Client::new();
    let res = client.post(url).multipart(form).send();

    match res {
        Ok(response) => {
            if response.status().is_success() {
                spinner.finish_and_clear();
                println!("✅ {}", "File uploaded successfully!".green());
                devnet_register_program_with_coprocessor(None, None);
                return true;
            } else {
                spinner.finish_and_clear();
                println!(
                    "Error uploading file: {}",
                    response.text().expect("Error unwrapping response")
                );
                return false;
            }
        }
        Err(e) => {
            spinner.finish_and_clear();
            if e.to_string()
                .contains("request or response body error for url")
            {
                println!(
                    "❌ {}",
                    "Devnet container inactive, Please run the start-devnet command then try again!"
                        .red()
                );
            }
            return false;
        }
    }
}

/// @notice This function is a recursive function that checks the status of devnet_register_program_with_coprocessor function and
/// recalls the function for a maximum of 5 times with a 5 seconds interval if a positive result is not obtained from the devnet_register_program_with_coprocessor function.
/// @param response This is the response gotten from the devnet_register_program_with_coprocessor function
/// @param retries this tracks the number of times the recursion has happened so we dont exceed 5
/// @param spinner This is an instance of a spinner displayed to users to signify the running process.
fn check_and_recal_devnet_solver_register(
    response: String,
    spinner: Option<ProgressBar>,
    retries: Option<u8>,
) {
    if response.contains("ready") {
        let possible_machine_hash = display_machine_hash();
        if let Some(machine_hash) = possible_machine_hash {
            println!(
                "✅ {} {} {}",
                "Successfully published your program".green(),
                "with machine hash".green(),
                machine_hash.green()
            );
        } else {
            println!("✅ {}", "Successfully published your program".green());
        }
        println!("✅ {} {}", "RESPONSE::".green(), response.green());
        return;
    } else {
        std::thread::sleep(time::Duration::from_secs(5));
        let mut retries_count: u8 = 0;

        if let Some(retries) = retries {
            if retries >= 5 {
                println!(
                    "❌ {} {} {}",
                    "Solver failed to finish setup after ".red(),
                    retries,
                    "retries".red()
                );
                println!("{}", response.red());
                return;
            }
            retries_count = retries;
        }

        if let Some(mut new_spinner) = spinner {
            new_spinner.finish_and_clear();
            new_spinner = get_spinner();
            new_spinner.set_message("Waiting for solver to finish publication process...");

            devnet_register_program_with_coprocessor(Some(new_spinner), Some(retries_count + 1));
        } else {
            let new_spinner = get_spinner();
            new_spinner.set_message("Waiting for solver to finish publication process...");
            devnet_register_program_with_coprocessor(Some(new_spinner), Some(retries_count + 1));
        }
    }
}
