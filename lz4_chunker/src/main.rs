mod chunker;
mod merger;

use std::error::Error;
use chunker::chunk_lz4_file;
use merger::merge_lz4_chunks;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <chunk|merge> [args...]", args[0]);
        eprintln!("  chunk <input.lz4> <output_prefix>");
        eprintln!("  merge <prefix> <output.lz4>");
        std::process::exit(1);
    }
    
    let mode = &args[1];
    
    let result = match mode.as_str() {
        "chunk" => {
            if args.len() != 4 {
                eprintln!("Usage: {} chunk <input.lz4> <output_prefix>", args[0]);
                std::process::exit(1);
            }
            chunk_command(&args[2], &args[3])
        }
        "merge" => {
            if args.len() != 4 {
                eprintln!("Usage: {} merge <prefix> <output.lz4>", args[0]);
                std::process::exit(1);
            }
            merge_command(&args[2], &args[3])
        }
        _ => {
            eprintln!("Unknown mode: {}. Use 'chunk' or 'merge'.", mode);
            std::process::exit(1);
        }
    };
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn chunk_command(input: &str, prefix: &str) -> Result<(), Box<dyn Error>> {
    println!("Chunking {} -> {}...", input, prefix);
    let start = std::time::Instant::now();
    
    let chunks = chunk_lz4_file(input, prefix)?;
    
    let total_elapsed = start.elapsed().as_millis();
    println!("✓ Created {} chunks in {}ms", chunks.len(), total_elapsed);
    
    for chunk in chunks {
        println!("  [{}] offset={} size={} bytes", 
                 chunk.index, chunk.byte_offset, chunk.compressed_size);
    }
    
    Ok(())
}

fn merge_command(prefix: &str, output: &str) -> Result<(), Box<dyn Error>> {
    println!("Merging {} -> {}...", prefix, output);
    let start = std::time::Instant::now();
    
    merge_lz4_chunks(prefix, output)?;
    
    let total_elapsed = start.elapsed().as_millis();
    println!("✓ Merged in {}ms", total_elapsed);
    
    Ok(())
}
