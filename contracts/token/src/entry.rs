use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{ckb_constants::Source, ckb_types::{bytes::Bytes, prelude::*}, debug, high_level::{QueryIter, load_cell_data, load_cell_lock_hash, load_cell_type_hash, load_script}};

use crate::error::Error;

// u128 is 16 bytes
const UDT_LEN: usize = 16;
const SCRIPT_LEN: usize = 32;

fn check_owner_mode(args: &Bytes) -> Result<(), Error> {
    let mut owner = [0u8; SCRIPT_LEN];
    owner.copy_from_slice(&args[..SCRIPT_LEN]);
    
    for i in 0.. {
        match load_cell_lock_hash(i, Source::Input) {
            Ok(data) => {
                if owner == data {
                    return Ok(());
                }
            }
            Err(_) => {
                break;
            }
        };
    }

    for i in 0.. {
        match load_cell_type_hash(i, Source::Input) {
            Ok(data) => {
                if Some(owner) == data {
                    return Ok(());
                }
            }
            Err(_) => {
                break;
            }
        };
    }

    for i in 0.. {
        match load_cell_type_hash(i, Source::Output) {
            Ok(data) => {
                if Some(owner) == data {
                    return Ok(());
                }
            }
            Err(_) => {
                break;
            }
        };
    }

    return Err(Error::ErrorMode);
}

fn collect_amount(so: Source) -> Result<u128, Error> {
    let mut buf = [0u8; UDT_LEN];
    let mut sum: u128 = 0;

    QueryIter::new(load_cell_data, so).for_each(|data| {
        assert!(data.len() >= UDT_LEN);
        buf.copy_from_slice(&data[..UDT_LEN]);
        let v = u128::from_le_bytes(buf);
        assert!(sum + v > sum);
        assert!(sum + v >= v);
        sum += v;
    });
    Ok(sum)
}

pub fn main() -> Result<(), Error> {
    let inputs_amount = collect_amount(Source::GroupInput)?;
    let outputs_amount = collect_amount(Source::GroupOutput)?;

    if inputs_amount == outputs_amount {
        return Ok(());
    }

    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:?}", args);

    // return an error if args is invalid
    if args.is_empty() {
        return Err(Error::ErrorArgs);
    }

    // owner mode
    return check_owner_mode(&args);
}
