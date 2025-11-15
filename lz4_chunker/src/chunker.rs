use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

const TARGET_CHUNK_SIZE: usize = 5 * 1024 * 1024; // 5 MB target

#[derive(Clone, Debug)]
pub struct CompressedChunkInfo {
    pub index: u64,
    pub byte_offset: u64,
    pub compressed_size: usize,
    pub uncompressed_estimate: Option<usize>,
}

/// Read 4 bytes as little-endian u32
fn read_u32_le(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

/// Chunk an LZ4 file with size-prepended blocks (compress_prepend_size format)
pub fn chunk_lz4_file(input: &str, out_prefix: &str) -> Result<Vec<CompressedChunkInfo>, Box<dyn Error>> {
    let start_time = std::time::Instant::now();
    
    let input_file = File::open(input)?;
    let mut reader = BufReader::new(input_file);
    
    let mut all_data = Vec::new();
    reader.read_to_end(&mut all_data)?;
    
    if all_data.is_empty() {
        return Err("File is empty".into());
    }
    
    let mut chunks = Vec::new();
    let mut chunk_index = 1u64;
    let mut offset = 0;
    let mut chunk_start = 0;
    let mut chunk_compressed = 0;
    let mut chunk_blocks = Vec::new();
    
    // Parse size-prepended blocks: [4-byte size LE][compressed data]
    while offset + 4 <= all_data.len() {
        let block_size = read_u32_le(&all_data[offset..offset + 4]) as usize;
        let block_total = 4 + block_size;
        
        if block_size == 0 || offset + block_total > all_data.len() {
            break;
        }
        
        chunk_blocks.push((offset, block_total));
        chunk_compressed += block_total;
        offset += block_total;
        
        // Chunk if we've reached target size
        if chunk_compressed >= TARGET_CHUNK_SIZE {
            let out_file = format!("{}.{:04}.lz4", out_prefix, chunk_index);
            write_chunk_file(
                &out_file,
                &all_data,
                &chunk_blocks,
            )?;
            
            chunks.push(CompressedChunkInfo {
                index: chunk_index,
                byte_offset: chunk_start as u64,
                compressed_size: chunk_compressed,
                uncompressed_estimate: None,
            });
            
            chunk_index += 1;
            chunk_start = offset;
            chunk_compressed = 0;
            chunk_blocks.clear();
        }
    }
    
    // Write final chunk with remaining blocks
    if !chunk_blocks.is_empty() {
        let out_file = format!("{}.{:04}.lz4", out_prefix, chunk_index);
        write_chunk_file(
            &out_file,
            &all_data,
            &chunk_blocks,
        )?;
        
        chunks.push(CompressedChunkInfo {
            index: chunk_index,
            byte_offset: chunk_start as u64,
            compressed_size: chunk_compressed,
            uncompressed_estimate: None,
        });
    }
    
    let elapsed = start_time.elapsed().as_millis();
    eprintln!("Chunking completed: {} chunks in {}ms", chunks.len(), elapsed);
    
    Ok(chunks)
}

/// Write a single chunk file (just concatenate blocks verbatim)
fn write_chunk_file(
    out_path: &str,
    data: &[u8],
    blocks: &[(usize, usize)],
) -> Result<(), Box<dyn Error>> {
    let out_file = File::create(out_path)?;
    let mut writer = BufWriter::new(out_file);
    
    // Write blocks verbatim
    for &(block_start, block_size) in blocks {
        writer.write_all(&data[block_start..block_start + block_size])?;
    }
    
    writer.flush()?;
    Ok(())
}
