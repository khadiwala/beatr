use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::io::BufReader;



fn main() {
    let FORMAT: &[u8] = "BPS1".as_bytes();

    println!("Hello, world!");
    let mut file = File::open("test.bps").unwrap();
    let mut reader = BufReader::new(file);

    let mut version = [0 as u8; 4];
    reader.read_exact(&mut version);
    if FORMAT != version {
        panic!("bytes {:?} not a valid bps header", &version);
    }
    let sourcesize = decode(&mut reader).unwrap();
    let targetsize = decode(&mut reader).unwrap();
    let metadatasize = decode(&mut reader).unwrap();
    println!("{}, {}, {}", sourcesize, targetsize, metadatasize);
}

fn encode<T : Write>(mut src: u64, dst: &mut Write) {
    loop {
        let val: u8 = (src as u8) & 0x7F;
        src = src >> 7;
        if src == 0 {
            dst.write_all(&[val | 0x80]).unwrap();
            break;
        }
        dst.write_all(&[val]).unwrap();
        src -= 1;
    }
}

fn decode<T : Read>(src: &mut T) -> Result<u64, io::Error> {
    let mut data: u64 = 0;
    let mut shift: u64 = 1;
    for b in src.bytes() {
        let x: u64 = b? as u64;
        data += (x & 0x7F) * shift;
        if x & 0x80 != 0 {
            break;
        }
        shift = shift << 7;
        data += shift;
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test1byte() {
        for i in 0..(1 << 7 - 1) {
            enc_dec_num(i);
        }
    }

    #[test]
    fn testmaxbyte() {
        enc_dec_num(1 << 7);
    }

    #[test]
    fn test2byte() {
        enc_dec_num(1 << 9);
    }

    #[test]
    fn test3byte() {
        enc_dec_num(1 << 17 + 37);
    }

    fn enc_dec_num(num: u64) {
        let mut dst = Vec::new();
        encode(num, &mut dst);
        assert_eq!(decode(dst.as_slice()).unwrap(), num);
    }
}


