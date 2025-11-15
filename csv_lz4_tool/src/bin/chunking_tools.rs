use std::error::Error;
use std::fs::File;
use std::io::Write;

use csv_lz4_tool::chunking::{chunk_file, DEFAULT_CHUNK_SIZE};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        println!("Usage:");
        println!("  chunk_tool <input_file> <output_folder>");
        println!("Example:");
        println!("  chunk_tool input.lz4 chunks/");
        return Ok(());
    }

    let input = &args[1];
    let output_folder = &args[2];

    println!("🔍 Chunking file: {}", input);

    // Ensure output dir exists
    std::fs::create_dir_all(output_folder)?;

    let chunks = chunk_file(input, DEFAULT_CHUNK_SIZE)?;
    println!("📦 Total chunks created: {}", chunks.len());

    for c in chunks {
        let out_path = format!("{}/chunk_{}.bin", output_folder, c.index);
        let mut out = File::create(out_path)?;
        out.write_all(&c.data)?;
    }

    println!("✅ Chunking complete!");
    Ok(())
}
