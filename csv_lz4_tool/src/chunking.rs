use std::fs::File;
use std::io::{BufReader, Read};
use std::error::Error;

pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024; // 64 KB

#[derive(Debug)]
pub struct Chunk {
    pub index: usize,
    pub data: Vec<u8>,
}

/// Reads a file and splits it into fixed-size chunks.
/// RETURNS: Vec<Chunk>
pub fn chunk_file(path: &str, chunk_size: usize) -> Result<Vec<Chunk>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut chunks = Vec::new();
    let mut buffer = vec![0u8; chunk_size];
    let mut index = 0;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }

        chunks.push(Chunk {
            index,
            data: buffer[..n].to_vec(),
        });

        index += 1;
    }

    Ok(chunks)
}
