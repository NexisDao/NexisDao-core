// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{QueryIter, load_cell_data, load_cell_lock_hash, load_cell_type_hash, load_script},
};

use crate::error::Error;
use config::{config::ID, def_get_config_data, require};
def_get_config_data!();

// u128 is 16 bytes
const UDT_VAL_LEN: usize = 16;
const SCRIPT_LEN: usize = 32;

// udt.hash, interest, rate
fn get_udt_hash(conf: &Bytes, id: u32) -> Result<[u8; 32], Error> {
    let udt_data = get_config_data(conf.to_vec(), id)?;
    let mut udt_hash_buf = [0u8; 32];
    udt_hash_buf.copy_from_slice(&udt_data[..SCRIPT_LEN]);

    Ok(udt_hash_buf)
}

/*
burn TAI, destroy sys_debt
*/
fn do_offset(args: &Bytes, data: Vec<u8>) -> Result<(), Error> {
    load_cell_data(0, Source::GroupOutput).expect_err("exist sys debt(output)");
    require!(data.len() >= UDT_VAL_LEN);
    let mut buf = [0u8; UDT_VAL_LEN];
    buf.copy_from_slice(&data[..UDT_VAL_LEN]);
    let debt_amount = u128::from_le_bytes(buf);

    let tai_hash = get_udt_hash(args, ID::TAIType as u32)?;
    let tai_hash1 = load_cell_type_hash(0, Source::Output)?;
    require!(tai_hash1 == Some(tai_hash));

    let burn_lock = get_udt_hash(args, ID::BurnLock as u32)?;
    let lock0 = load_cell_lock_hash(0, Source::Output)?;
    require!(burn_lock == lock0);

    let data = load_cell_data(0, Source::Output)?;
    require!(data.len() >= UDT_VAL_LEN);
    let mut buf = [0u8; UDT_VAL_LEN];
    buf.copy_from_slice(&data[..UDT_VAL_LEN]);
    let v = u128::from_le_bytes(buf);
    require!(debt_amount == v);

    let mut exist_lock = false;
    QueryIter::new(load_cell_lock_hash, Source::Input).for_each(|lock| {
        if lock == burn_lock{
            exist_lock = true;
        }
    });
    require!(!exist_lock);

    Ok(())
}

fn new_sys_debt(args: &Bytes) -> Result<(), Error> {
    load_cell_data(0, Source::GroupInput).expect_err("exist sys_debt(input)");
    let data = load_cell_data(0, Source::GroupOutput)?;
    require!(data.len() >= UDT_VAL_LEN);

    let auction = get_udt_hash(args, ID::AuctionType as u32)?;
    let type_hash = load_cell_type_hash(1, Source::Input)?;
    require!(type_hash == Some(auction));

    let lock = get_udt_hash(args, ID::CommunityLock as u32)?;
    let lock0 = load_cell_lock_hash(0, Source::GroupOutput)?;
    require!(lock == lock0);

    let self_hash = load_cell_type_hash(0, Source::GroupOutput)?;
    let type_hash1 = load_cell_type_hash(1, Source::Output)?;
    require!(self_hash == type_hash1);

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:x?}", args);

    require!(!args.is_empty());

    load_cell_data(1, Source::GroupInput).expect_err("more than one sys_debt(input)");
    load_cell_data(1, Source::GroupOutput).expect_err("more than one sys_debt(output)");
    match load_cell_data(0, Source::GroupInput) {
        Ok(data) => {
            return do_offset(&args, data);
        }
        _ => {
            return new_sys_debt(&args);
        }
    }
}
