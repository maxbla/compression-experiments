use std::collections::{BinaryHeap, HashMap};
use std::cmp::{Reverse, Ordering};
use std::error::Error;

use std::fs::File;
use std::io::{Write, BufRead, BufReader, Seek, SeekFrom};

use bitvec::prelude::{BitVec, bitvec, BigEndian, LittleEndian};

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
    Interior(Count, HashMap<char, BitVec<LittleEndian, u8>>)
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

fn count_chars(r: &mut impl BufRead) -> Result<HashMap<char, Count>, Box<dyn Error>> {
    let mut frequencies = HashMap::new();
    for line in r.lines() {
        let line = line?;
        for character in line.chars() {
            match frequencies.get_mut(&character) {
                None => {frequencies.insert(character, 0);}
                Some(freq) => *freq += 1,
            };
        }
    }
    Ok(frequencies)
}

fn char_count_to_huffman_encoding(char_count: HashMap<char, Count>) ->
    HashMap<char, BitVec<LittleEndian, u8>> 
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

fn serialize_huffman_encoding(encoding: &HashMap<char, BitVec<LittleEndian, u8>>) -> Vec<u8> {
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
    buffer.push(b'\n');
    buffer
}

fn encode<R, W>(mut r: R, mut out: W) -> Result<(), Box<dyn Error>> 
where R: BufRead + Seek, W:Write
{
    let char_count = count_chars(&mut r)?;
    let encoding = char_count_to_huffman_encoding(char_count);
    let serialized_encoding: Vec<u8> = serialize_huffman_encoding(&encoding);
    out.write(&serialized_encoding)?;
    r.seek(SeekFrom::Start(0))?;

    let mut buffer = encoding!();
    for line in r.lines() {
        let line = line?;
        for character in line.chars() {
            let code = encoding.get(&character).unwrap();
            let mut code = code.clone();
            buffer.append(&mut code);
        }
        let slice = buffer.into_boxed_slice();
        out.write(&slice[..slice.len()-1])?;
        buffer = BitVec::from_element(slice[slice.len()-1]);
    }
    let bytes:Vec<u8> = buffer.into();
    out.write(&bytes[..])?;
    Ok(())
}

fn decode(mut r: impl BufRead) -> Result<String, Box<dyn Error>> {
    let mut huffman_encoding: HashMap<BitVec<LittleEndian, u8>, char> = HashMap::new();
    // parse huffman encoding for each character
    let mut line = String::new();
    while let Ok(_size) = r.read_line(&mut line) {
        line.pop(); // remove trailing '\n'
        let mut chars = line.chars();
        let encoded_char = match chars.next() {
            Some(character) => character,
            None => break // empty line -> end of encoding section
        };
        //println!("encoded_char: {}", encoded_char);
        let mut encoding = encoding!();
        for bit in chars {
            //println!("bit: {}", bit);
            match bit {
                '0' => encoding.push(false),
                '1' => encoding.push(true),
                _ => panic!()  // TODO: return Err(ParseError)
            }
        }
        line.clear();
        huffman_encoding.insert(encoding, encoded_char);
    }
    // parse text of file
    let mut ret = String::new();
    let mut bytes = r.bytes();
    let mut bit_buffer:BitVec<LittleEndian, u8> = BitVec::new();
    let mut to_encode: BitVec<LittleEndian, u8> = BitVec::new();
    while let Some(byte) = bytes.next() {
        let byte = byte?;
        println!("reading byte: {}", byte);
        let mut tmp: BitVec<BigEndian, u8> = BitVec::from_element(byte);
        bit_buffer.append(&mut tmp);
        while let Some(bit) = bit_buffer.pop() {
            //println!("reading bit: {}", bit);
            to_encode.push(bit);
            if to_encode.len() > 100 {
                panic!("to_encode too long!")
            }
            match huffman_encoding.get(&to_encode) {
                None => {;},
                Some(character) => {
                    ret.push(*character);
                    to_encode.clear();
                }
            }
        }
    }
    Ok(ret)
}

fn main() -> Result<(), Box<dyn Error>> {
    let uncompressed_path = "/home/max/code/rust/compression/src/Grimms";
    let compressed_path = "/home/max/code/rust/compression/src/Grimms.huffman";
    let book = BufReader::new(File::open(uncompressed_path)?);
    let out = File::create(compressed_path)?;
    encode(book, out)?;
    // let char_count = count_chars(&mut book)?;
    // let encoding = char_count_to_huffman_encoding(char_count);
    // drop(book);

    // let book = BufReader::new(File::open(path).expect("File not found"));
    // let mut buffer = encoding!();
    // for line in book.lines() {
    //     let line = line?;
    //     for character in line.chars() {
    //         let code = encoding.get(&character).unwrap();
    //         buffer.append(&mut code.clone());
    //     }
    // }
    // let mut file = File::create("/home/max/code/rust/compression/src/Grimms.huffman")
    //     .expect("Could not create file");
    // file.write(&serialize_huffman_encoding(&encoding)[..])?;
    // let bytes:Vec<u8> = buffer.into();
    // file.write(&bytes[..])?;

    let compressed = BufReader::new(File::open(compressed_path)?);
    let decoded = decode(compressed)?;
    println!("{}", &decoded[0..100]);

    Ok(())
}
