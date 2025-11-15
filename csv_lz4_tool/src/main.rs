use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

const CHUNK_SIZE: usize = 64 * 1024; // 64 KB chunks

/// Compress a CSV file using LZ4
fn compress_csv(input: &str, output: &str) -> Result<(), Box<dyn Error>> {
    let input_file = File::open(input)?;
    let output_file = File::create(output)?;
    
    let mut reader = BufReader::new(input_file);
    let mut writer = BufWriter::new(output_file);
    
    let mut chunk = vec![0u8; CHUNK_SIZE];
    
    loop {
        let bytes_read = reader.read(&mut chunk)?;
        if bytes_read == 0 {
            break;
        }
        
        // Compress the chunk with size prepended
        let compressed = lz4_flex::compress_prepend_size(&chunk[..bytes_read]);
        writer.write_all(&compressed)?;
    }
    
    writer.flush()?;
    Ok(())
}

/// Decompress a CSV file using LZ4
fn decompress_csv(input: &str, output: &str) -> Result<(), Box<dyn Error>> {
    let input_file = File::open(input)?;
    let output_file = File::create(output)?;
    
    let mut reader = BufReader::new(input_file);
    let mut writer = BufWriter::new(output_file);
    
    let mut all_data = Vec::new();
    reader.read_to_end(&mut all_data)?;
    
    // Decompress all chunks
    let mut offset = 0;
    while offset < all_data.len() {
        let decompressed = lz4_flex::decompress_size_prepended(&all_data[offset..])?;
        writer.write_all(&decompressed)?;
        
        // Move offset forward by compressed size
        // The size is stored in the first 4 bytes of the compressed data
        if offset + 4 > all_data.len() {
            break;
        }
        let compressed_size = u32::from_le_bytes([
            all_data[offset],
            all_data[offset + 1],
            all_data[offset + 2],
            all_data[offset + 3],
        ]) as usize;
        offset += 4 + compressed_size;
    }
    
    writer.flush()?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 4 {
        eprintln!("Usage: {} <compress|decompress> <input> <output>", args[0]);
        eprintln!("  Example: {} compress input.csv output.lz4", args[0]);
        eprintln!("  Example: {} decompress input.lz4 output.csv", args[0]);
        std::process::exit(1);
    }
    
    let mode = &args[1];
    let input = &args[2];
    let output = &args[3];
    
    let result = match mode.as_str() {
        "compress" => {
            println!("Compressing {} to {}...", input, output);
            compress_csv(input, output)
        }
        "decompress" => {
            println!("Decompressing {} to {}...", input, output);
            decompress_csv(input, output)
        }
        _ => {
            eprintln!("Unknown mode: {}. Use 'compress' or 'decompress'.", mode);
            std::process::exit(1);
        }
    };
    
    match result {
        Ok(_) => println!("Done!"),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
