fn main() {
    println!("cargo:rerun-if-changed=.env");

    // Load the .env file
    match dotenvy::dotenv() {
        Ok(_) => println!("cargo:warning=Successfully loaded .env file"),
        Err(e) => println!("cargo:warning=Failed to load .env file: {}", e),
    }

    // Iterate through all environment variables
    for (key, value) in std::env::vars() {
        // Export environment variables to the build process
        println!("cargo:rustc-env={}={}", key, value);
    }
}
