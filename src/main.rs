use std::collections::{BinaryHeap, HashMap};
use std::cmp::{Reverse, Ordering};
use std::error::Error;
use std::fmt::Display;

use std::fs::File;
use std::io::{Write, BufRead, Read, BufReader, Seek, SeekFrom, ErrorKind};

use bitvec::prelude::{bitvec, LittleEndian};

macro_rules! encoding {
    ($x:expr) => {
        bitvec![LittleEndian, u8; $x]
    };
    () => {
        bitvec![LittleEndian, u8;]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HuffmanNode {
    Leaf(Count, char),
    Interior(Count, HashMap<char, BitVec>)
}

impl Ord for HuffmanNode {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (HuffmanNode::Leaf(count1, _), HuffmanNode::Leaf(count2, _)) => {
                count1.cmp(count2)
            }
            (HuffmanNode::Leaf(count1, _), HuffmanNode::Interior(count2, _)) => {
                count1.cmp(count2)
            }
            (HuffmanNode::Interior(count1, _), HuffmanNode::Leaf(count2, _)) => {
                count1.cmp(count2)
            }
            (HuffmanNode::Interior(count1, _), HuffmanNode::Interior(count2, _)) => {
                count1.cmp(count2)
            }
        }
    }
}

impl PartialOrd for HuffmanNode {
    fn partial_cmp(&self, other:&Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
struct HuffmanEncodingError {
    /// The bitpattern that was last tried before failing
    bitpattern: BitVec
}

impl HuffmanEncodingError {
    fn new(bitpattern: BitVec) -> HuffmanEncodingError {
        HuffmanEncodingError{bitpattern}
    }
}

impl Display for HuffmanEncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "HuffmanEndocingError: {}", self.bitpattern)
    }
}

impl Error for HuffmanEncodingError {
}

/// combines two Huffman Nodes, updating encodings 
/// for every character, and updating the total character count
/// the left subtree gets an added 0 to the encoding and the right a 1
fn combine(left: HuffmanNode, right: HuffmanNode) -> HuffmanNode {
    match (left, right) {
        (HuffmanNode::Leaf(count1, char1), HuffmanNode::Leaf(count2, char2)) => {
            let mut encoding = HashMap::new();
            encoding.insert(char1, encoding!(0));
            encoding.insert(char2, encoding!(1));
            HuffmanNode::Interior(count1 + count2, encoding)
        }
        (HuffmanNode::Leaf(count1, char1), HuffmanNode::Interior(count2, mut encoding)) => {
            for (_, code) in encoding.iter_mut() {
                code.insert(0, true);
            }
            encoding.insert(char1, encoding!(0));
            HuffmanNode::Interior(count1 + count2, encoding)
        }
        (HuffmanNode::Interior(count1, mut encoding), HuffmanNode::Leaf(count2, char1)) => {
            for (_, code) in encoding.iter_mut() {
                code.insert(0, false);
            }
            encoding.insert(char1, encoding!(1));
            HuffmanNode::Interior(count1 + count2, encoding)
        }
        (HuffmanNode::Interior(count1, mut encoding1), HuffmanNode::Interior(count2, encoding2)) => {
            for (_, code) in encoding1.iter_mut() {
                code.insert(0, false);
            }
            for (character, mut code) in encoding2 {
                code.insert(0, true);
                encoding1.insert(character, code);
            }
            HuffmanNode::Interior(count1 + count2, encoding1)
        }
    }
}

/// type used to store count of characters
/// u32 should be sufficient, but if there is overflow
/// u46, u128 and num::BigInt can be used
type Count = u32;

type BitVec = bitvec::prelude::BitVec<LittleEndian, u8>;

fn count_chars(r: &mut impl BufRead) -> Result<HashMap<char, Count>, Box<dyn Error>> {
    let mut frequencies = HashMap::new();
    let mut num_lines = 0;
    for (line_number, line) in r.lines().enumerate() {
        let line = line?;
        num_lines = line_number;
        for character in line.chars() {
            match frequencies.get_mut(&character) {
                None => {frequencies.insert(character, 0);}
                Some(freq) => *freq += 1,
            };
        }
    }
    frequencies.insert('\n', num_lines as Count); //TODO: cast properly
    Ok(frequencies)
}

fn char_count_to_huffman_encoding(char_count: HashMap<char, Count>) ->
    HashMap<char, BitVec> 
{
    let mut huffman_heap:BinaryHeap<_> = char_count
        .into_iter()
        .fold(BinaryHeap::new(), |mut heap, (character, frequency)| {
            heap.push(Reverse(HuffmanNode::Leaf(frequency, character)));
            heap
        });
    while huffman_heap.len() > 1 {
        let node1 = huffman_heap.pop().unwrap().0;
        let node2 = huffman_heap.pop().unwrap().0;
        let combined = combine(node1, node2);
        huffman_heap.push(Reverse(combined));
    }
    match huffman_heap.pop().unwrap().0 {
        HuffmanNode::Interior(_total_chars, encoding) => {
            encoding
        },
        HuffmanNode::Leaf(_total_chars, character) => {
            let mut encoding = HashMap::new();
            encoding.insert(character, encoding!());
            encoding
        }
    }
}

fn serialize_huffman_encoding(encoding: &HashMap<char, BitVec>) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::with_capacity(encoding.len());
    let mut utf8_buffer = [0_u8; 4];
    let mut encoding: Vec<_> = encoding.into_iter().collect();
    encoding.sort();
    for (character, code) in encoding {
        let utf8_slice = character.encode_utf8(&mut utf8_buffer).as_bytes();
        buffer.extend(utf8_slice.iter());
        for bit in code {
            buffer.push(if bit {b'1'} else {b'0'});
        }
        buffer.push(b'\n');
    }
    buffer
}

fn encode<R, W>(mut r: R, mut out: W) -> Result<(), Box<dyn Error>> 
where R: BufRead + Seek, W:Write
{
    let char_count = count_chars(&mut r)?;
    let encoding = char_count_to_huffman_encoding(char_count);
    let serialized_encoding: Vec<u8> = serialize_huffman_encoding(&encoding);
    out.write(&serialized_encoding)?;
    out.write(b"\n\n")?; //separation between encoding and body of text
    r.seek(SeekFrom::Start(0))?;

    let mut buffer = encoding!();
    for line in r.lines() {
        let line = line?;
        for character in line.chars() {
            let code = encoding.get(&character).unwrap();
            let mut code = code.clone();
            buffer.append(&mut code);
        }
        buffer.append(&mut encoding.get(&'\n').unwrap().clone());
        if buffer.len() > 8 {
            //split off incomplete byte from buffer
            let split_index = buffer.len() - buffer.len() % 8;
            let buffer_remainder = buffer.split_off(split_index);
            
            let slice = buffer.into_boxed_slice();
            out.write(&slice)?;
            buffer = buffer_remainder;
        }
    }
    let bytes:Vec<u8> = buffer.into();
    out.write(&bytes[..])?;
    Ok(())
}

fn read_utf8<R: Read>(r:&mut R) -> Result<char, Box<dyn Error>> {
    let mut buffer = [0_u8;4];
    r.read_exact(&mut buffer[0..1])?;
    let num_bytes = if buffer[0] & 0b10000000u8 == 0 {
        1
    } else if buffer[0] & 0b11110000u8 == 0b11110000u8 {
        4
    } else if buffer[0] & 0b11100000u8 == 0b11100000u8 {
        3
    } else if buffer[0] & 0b11000000u8 == 0b11000000u8 {
        2
    } else {
        return Err(Box::new(std::io::Error::new(ErrorKind::InvalidData, "invalid utf-8")));
    };
    let utf8_slice = &mut buffer[0..num_bytes];
    r.read_exact(&mut utf8_slice[1..])?;
    let string = std::str::from_utf8(utf8_slice)?;
    let ch = string.chars().next().unwrap();
    Ok(ch)
}

// read one line (slowly) by reading single bytes at a time
fn read_line<R:Read>(r:&mut R, buf: &mut String) -> Result<usize, Box<dyn Error>> {
    let mut ch = ' ';
    let mut num_read:usize = 0;
    while ch != '\n' {
        ch = read_utf8(r)?;
        buf.push(ch);
        num_read += 1;
    };
    Ok(num_read)
} 

fn decode<R, W>(mut r: R, mut out: W) -> Result<(), Box<dyn Error>>
where R: Read + Seek, W:Write
{
    let mut huffman_encoding: HashMap<BitVec, char> = HashMap::new();
    // parse huffman encoding for each character
    let mut line = String::new();
    loop {
        read_line(&mut r, &mut line)?;
        // for use when encoding the '\n' character itself
        let mut spare_line = String::new();
        line.pop(); // remove trailing '\n'
        let mut chars = line.chars();
        let encoded_char = match chars.next() {
            Some(character) => character,
            None => { // this was an empty line
                read_line(&mut r, &mut spare_line)?;
                if spare_line == "\n" {
                    break // two empty lines -> end of encoding section
                }
                spare_line.pop();
                chars = spare_line.chars();
                '\n'
            }
        };
        let mut encoding = encoding!();
        for bit in chars {
            match bit {
                '0' => encoding.push(false),
                '1' => encoding.push(true),
                _ => return Err(Box::new(HuffmanEncodingError::new(encoding)))
            }
        }
        line.clear();
        huffman_encoding.insert(encoding, encoded_char);
    }
    // parse text of file
    let mut bytes = r.bytes();
    let mut bit_buffer:BitVec = BitVec::new();
    let mut to_encode: BitVec = BitVec::new();
    while let Some(byte) = bytes.next() {
        let byte = byte?;
        let mut tmp: BitVec = BitVec::from_element(byte);
        tmp.reverse();
        bit_buffer.append(&mut tmp);
        while let Some(bit) = bit_buffer.pop() {
            to_encode.push(bit);
            if to_encode.len() > 100 { //TODO: put meaningful number here
                // one character is 1/(2^100) times less likely to appear
                // than another character the source text had over 10^30 
                // characters -> 10^18 TB
                return Err(Box::new(HuffmanEncodingError::new(to_encode)))
            }
            match huffman_encoding.get(&to_encode) {
                None => {;},
                Some(character) => {
                    // can store one utf8 encoded character
                    let mut utf8_buffer = [0_u8;4];
                    let encoded = character.encode_utf8(&mut utf8_buffer);
                    out.write(encoded.as_bytes())?;
                    to_encode.clear();
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let uncompressed_path = "./test/Grimms";
    let compressed_path = "./test/Grimms.huffman";
    let book = BufReader::new(File::open(uncompressed_path)?);
    let out = File::create(compressed_path)?;
    encode(book, out)?;

    let compressed = File::open(compressed_path)?;
    let out = File::create("./test/Grimms.decompressed")?;
    decode(compressed, out)?;

    Ok(())
}
