use log::{error, info};
use protobuf_to_zod::generator::zod_generator::generate_zod_schemas;
use protobuf_to_zod::parser::parse_proto_file;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut proto_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    proto_path.push("files");
    proto_path.push("logdservice.proto");

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
    println!("Generated Zod schemas:\n{}", zod_schemas);

    Ok(())
}
