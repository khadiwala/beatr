use std::env;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;

use crc::crc32;

const FORMAT: &[u8] = b"BPS1";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        println!("usage: beatr <patch> <original> <output>");
    }

    let patch: Vec<u8> = slurp(&args[1]).unwrap();
    let src: Vec<u8> = slurp(&args[2]).unwrap();
    let target = apply_patch(&patch, &src).unwrap();

    let mut out = File::create(&args[3]).unwrap();
    out.write_all(&target).unwrap();
}

fn slurp(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut v = Vec::new();
    file.read_to_end(&mut v)?;
    Ok(v)
}

fn apply_patch(patch: &[u8], src: &[u8]) -> Result<Vec<u8>, String> {
    if FORMAT != &patch[0..4] {
        return Err(format!("Not a valid bps header: {:?}", &patch[0..4]));
    }
    let remaining = &patch[4..];
    let (source_sz, remaining) = decodenum(remaining)?;
    let (target_sz, remaining) = decodenum(remaining)?;
    let (metadata_sz, remaining) = decodenum(remaining)?;
    let mut target = Vec::with_capacity(target_sz as usize);
    if metadata_sz > (i64::max_value() as u64) {
        return Err(format!("illegal metadata size {}", metadata_sz));
    }
    let mut remaining = &remaining[metadata_sz as usize..];
    if (source_sz as usize) != src.len() {
        return Err(format!("patch does not apply to input file, expected length: {}, actual: {}",
                           source_sz, src.len()));
    }

    let mut source_relative_offset: usize = 0;
    let mut target_relative_offset: usize = 0;
    while remaining.len() > 12 {
        let (data, r) = decodenum(remaining)?;
        remaining = r;
        match action(data) {
            Action::SourceRead(length) => {
                let pos = target.len();
                target.extend(&src[pos..pos + length]);
            }
            Action::TargetRead(length) => {
                target.extend(&remaining[0..length]);
                remaining = &remaining[length..];
            }
            Action::SourceCopy(length) => {
                let (data, r) = decodenum(remaining)?;
                remaining = r;
                source_relative_offset = ((source_relative_offset as isize) + decode_signed(data)) as usize;
                let end = source_relative_offset + length;
                target.extend(&src[source_relative_offset..end]);
                source_relative_offset += length;
            }
            Action::TargetCopy(length) => {
                let (data, r) = decodenum(remaining)?;
                remaining = r;
                target_relative_offset = ((target_relative_offset as isize) + decode_signed(data)) as usize;
                for _i in 0..length {
                    target.push(target[target_relative_offset]);
                    target_relative_offset += 1;
                }
            }
        }
    }
    if remaining.len() != 12 {
        Err("invalid bps file".to_string())
    } else {
        verify_crc32(&remaining[0..4], crc32::checksum_ieee(&src))?;
        verify_crc32(&remaining[4..8], crc32::checksum_ieee(&target))?;
        verify_crc32(&remaining[8..12], crc32::checksum_ieee(&patch[0..patch.len() - 4]))?;
        Ok(target)
    }
}

fn verify_crc32(expected: &[u8], actual: u32) -> Result<(), String> {
    let (e0, e1, e2, e3) = (
        u32::from(expected[0]),
        u32::from(expected[1]),
        u32::from(expected[2]),
        u32::from(expected[3]));
    let expected = e0 | e1 << 8 | e2 << 16 | e3 << 24;
    if expected != actual {
        Err(format!("CRC doesn't match (expect: {}, actual: {})", expected, actual))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
enum Action {
    SourceRead(usize),
    TargetRead(usize),
    SourceCopy(usize),
    TargetCopy(usize),
}

fn action(data: u64) -> Action {
    let command = data & 0b11;
    let length = ((data >> 2) + 1) as usize;
    match command {
        0 => Action::SourceRead(length),
        1 => Action::TargetRead(length),
        2 => Action::SourceCopy(length),
        3 => Action::TargetCopy(length),
        _ => panic!("illegal command identifier")
    }
}

fn decodenum(src: &[u8]) -> Result<(u64, &[u8]), &'static str> {
    let mut data: u64 = 0;
    let mut shift: u64 = 1;
    for (i, b) in src.iter().enumerate() {
        let x = u64::from(*b);
        data += (x & 0x7F) * shift;
        if x & 0x80 != 0 {
            return Ok((data, &src[i + 1..]));
        }
        shift <<= 7;
        data += shift;
    }
    Err("Invalid bps file")
}

fn decode_signed(num: u64) -> isize {
    let signum: isize = match num & 1 {
        0 => 1,
        _ => -1
    };
    signum * ((num >> 1) as isize)
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

    fn encodenum(mut src: u64, dst: &mut Vec<u8>) {
        loop {
            let val: u8 = (src as u8) & 0x7F;
            src >>= 7;
            if src == 0 {
                dst.push(val | 0x80);
                return;
            }
            dst.push(val);
            src -= 1;
        }
    }

    fn enc_dec_num(num: u64) {
        let mut dst = Vec::new();
        encodenum(num, &mut dst);
        let (dec, rem) = decodenum(&dst).unwrap();
        assert_eq!(dec, num);
        assert_eq!(rem.len(), 0);
    }
}


