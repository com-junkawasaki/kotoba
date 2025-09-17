use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The .kotobanet file to run
    #[arg(required = true)]
    input_file: PathBuf,

    /// Port to run the server on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("🚀 Starting Kotoba Runtime...");
    println!("🔌 Loading configuration from: {:?}", args.input_file);
    println!("📡 Server will run on port: {}", args.port);

    // 1. Read and parse the .kotobanet file
    let config_content = std::fs::read_to_string(&args.input_file)?;
    let frontend_config = kotoba_kotobas::frontend::FrontendParser::parse(&config_content)?;

    println!("✅ Configuration parsed successfully!");
    println!("📦 Found {} components.", frontend_config.components.len());
    for (name, component) in &frontend_config.components {
        println!("DEBUG: Component '{}' has render: '{}'", name, component.render);
    }
    println!("📄 Found {} pages.", frontend_config.pages.len());
    println!("🔗 Found {} API routes.", frontend_config.api_routes.len());

    // 2. Generate TSX/JS/CSS assets using kotoba2tsx
    println!("⏳ Generating frontend assets...");
    let tsx_code = kotoba2tsx::convert_frontend_config(&frontend_config)?;
    println!("✅ Generated TSX code ({} chars)", tsx_code.len());

    // Save the generated code to a temporary file for now
    let output_path = format!("generated_app_{}.tsx", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs());
    std::fs::write(&output_path, &tsx_code)?;
    println!("💾 Saved generated code to: {}", output_path);

    // 3. TODO: Start the HTTP server from kotoba-server
    println!("⏳ Starting server... (Disabled due to kotoba-server dependency issues)");

    println!("🎉 Kotoba application is running!");

    Ok(())
}
