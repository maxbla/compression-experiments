use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use compression::{encode, decode};

fn main() -> Result<(), Box<dyn Error>> {
    let uncompressed_path = "./test/Grimms";
    let compressed_path = "./test/Grimms.huffman";
    let book = BufReader::new(File::open(uncompressed_path)?);
    let out = BufWriter::new(File::create(compressed_path)?);
    encode(book, out)?;

    let compressed = BufReader::new(File::open(compressed_path)?);
    let out = BufWriter::new(File::create("./test/Grimms.decompressed")?);
    decode(compressed, out)?;

    Ok(())
}