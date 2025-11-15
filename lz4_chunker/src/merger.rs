use std::error::Error;
use std::fs::{File, metadata};
use std::io::{BufReader, BufWriter, Read, Write};

/// Read 4 bytes as little-endian u32
fn read_u32_le(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

/// Merge LZ4 chunks (size-prepended format) back into a single file
pub fn merge_lz4_chunks(prefix: &str, output: &str) -> Result<(), Box<dyn Error>> {
    let start_time = std::time::Instant::now();
    
    // Find all chunk files matching pattern: prefix.NNNN.lz4
    let mut chunks = Vec::new();
    for i in 1..=9999 {
        let chunk_path = format!("{}.{:04}.lz4", prefix, i);
        if metadata(&chunk_path).is_ok() {
            chunks.push(chunk_path);
        } else {
            break;
        }
    }
    
    if chunks.is_empty() {
        return Err("No chunk files found".into());
    }
    
    let output_file = File::create(output)?;
    let mut writer = BufWriter::new(output_file);
    
    for (idx, chunk_path) in chunks.iter().enumerate() {
        let input_file = File::open(chunk_path)?;
        let mut reader = BufReader::new(input_file);
        
        let mut chunk_data = Vec::new();
        reader.read_to_end(&mut chunk_data)?;
        
        if chunk_data.is_empty() {
            return Err(format!("Chunk {} is empty", idx + 1).into());
        }
        
        // Write chunk blocks verbatim
        writer.write_all(&chunk_data)?;
    }
    
    writer.flush()?;
    
    let elapsed = start_time.elapsed().as_millis();
    eprintln!("Merging completed: {} chunks in {}ms", chunks.len(), elapsed);
    
    Ok(())
}
