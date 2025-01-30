use log::{error, info};
use protobuf_to_zod::generator::zod_generator::generate_zod_schemas;
use protobuf_to_zod::parser::parse_proto_file;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Get the paths to the Protobuf file and output file from the command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "Usage: {} <path_to_proto_file> <path_to_output_file>",
            args[0]
        );
        std::process::exit(1);
    }
    let proto_path = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(&args[2]);

    info!("Reading Protobuf file from: {}", proto_path.display());

    let proto_content = fs::read_to_string(&proto_path).map_err(|e| {
        error!("Failed to read the proto file: {}", e);
        format!(
            "Failed to read the proto file '{}': {}",
            proto_path.display(),
            e
        )
    })?;

    info!("Parsing Protobuf file content");

    let proto_file = parse_proto_file(&proto_content).map_err(|e| {
        error!("Failed to parse Protobuf file: {}", e);
        format!("Failed to parse Protobuf file: {}", e)
    })?;

    info!("Successfully parsed Protobuf file");
    info!("Parsed content: {:#?}", proto_file);

    let zod_schemas = generate_zod_schemas(&proto_file);
    info!("Generated Zod schemas:\n{}", zod_schemas);

    // Write the generated Zod schemas to the output file
    fs::write(&output_path, format!("{}\n", zod_schemas)).map_err(|e| {
        error!("Failed to write to the output file: {}", e);
        format!(
            "Failed to write to the output file '{}': {}",
            output_path.display(),
            e
        )
    })?;

    info!(
        "Successfully wrote Zod schemas to: {}",
        output_path.display()
    );

    Ok(())
}
