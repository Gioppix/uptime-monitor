use std::{collections::HashMap, env};

fn main() {
    println!("cargo:rerun-if-changed=.env");

    let existing_vars: HashMap<String, String> = env::vars().collect();

    match dotenvy::dotenv() {
        Ok(_) => println!("cargo:warning=Successfully loaded .env file"),
        Err(e) => println!("cargo:warning=Failed to load .env file: {}", e),
    }

    for (key, value) in env::vars() {
        // Give precedence to existing env vars
        let value = existing_vars.get(&key).unwrap_or(&value);
        println!("cargo:rustc-env={}={}", key, value);

        // Rerun if variable changes (e.g. removing an override)
        println!("cargo::rerun-if-env-changed={key}");
    }
}
