mod chunker;

use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use chunker::chunk_lz4_file;

fn get_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn get_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <input.lz4> <output_prefix>", args[0]);
        eprintln!("  Chunks the LZ4 file into dynamically-sized segments");
        std::process::exit(1);
    }
    
    if let Err(e) = chunk_command(&args[1], &args[2]) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn chunk_command(input: &str, prefix: &str) -> Result<(), Box<dyn Error>> {
    let timestamp_start = get_timestamp();
    let timestamp_ms = get_timestamp_ms();
    
    println!("═══════════════════════════════════════════════════════════");
    println!("[CHUNKING START] Timestamp: {} | Time (ms): {}", timestamp_start, timestamp_ms);
    println!("Input:  {}", input);
    println!("Prefix: {}", prefix);
    println!("───────────────────────────────────────────────────────────");
    
    let start = std::time::Instant::now();
    let chunks = chunk_lz4_file(input, prefix)?;
    let total_elapsed = start.elapsed();
    
    let timestamp_end = get_timestamp();
    let timestamp_ms_end = get_timestamp_ms();
    
    println!();
    println!("✓ CHUNKING COMPLETE");
    println!("  Chunks Created:   {}", chunks.len());
    println!("  Duration:         {} ms ({:.3} sec)", total_elapsed.as_millis(), total_elapsed.as_secs_f64());
    println!();
    
    for chunk in chunks {
        println!("  [Chunk {}] offset={:10} size={:10} bytes", 
                 chunk.index, chunk.byte_offset, chunk.compressed_size);
    }
    
    println!();
    println!("[CHUNKING END]   Timestamp: {} | Time (ms): {}", timestamp_end, timestamp_ms_end);
    println!("═══════════════════════════════════════════════════════════");
    println!();
    
    Ok(())
}
