// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use crate::error::Error;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_data, load_cell_type_hash, load_script},
};
use config::{config::ID, def_get_config_data};
def_get_config_data!();

// u128 is 16 bytes
const UDT_VAL_LEN: usize = 16;
const SCRIPT_LEN: usize = 32;
const UDT_ID_LEN: usize = 4;

fn parse_data(data: Vec<u8>) -> (u128, u128, u32) {
    assert!(
        data.len() >= (2 * UDT_VAL_LEN + UDT_ID_LEN),
        "error debt data:{}",
        data.len()
    );
    let mut start: usize = 0;
    let mut end = UDT_VAL_LEN;
    let tai_amount_buf = &data[start..end];
    start = end;
    end = start + UDT_VAL_LEN;
    let udt_amount_buf = &data[start..end];
    start = end;
    end = start + UDT_ID_LEN;
    let udt_id_buf = &data[start..end];

    let mut buf_id = [0u8; UDT_ID_LEN];
    buf_id.copy_from_slice(udt_id_buf);
    let id = u32::from_le_bytes(buf_id);

    let mut buf = [0u8; UDT_VAL_LEN];
    buf.copy_from_slice(udt_amount_buf);
    let udt_amount = u128::from_le_bytes(buf);

    buf.copy_from_slice(tai_amount_buf);
    let tai_amount = u128::from_le_bytes(buf);

    (udt_amount, tai_amount, id)
}

// udt.hash, interest, rate
fn get_udt_hash(conf: &Bytes, id: u32) -> Result<[u8; 32], Error> {
    let udt_data = get_config_data(conf.to_vec(), id)?;
    let mut udt_hash_buf = [0u8; 32];
    udt_hash_buf.copy_from_slice(&udt_data[..SCRIPT_LEN]);

    Ok(udt_hash_buf)
}

fn liquidation(args: &Bytes) -> Result<(), Error> {
    load_cell_data(0, Source::GroupInput).expect_err("exist debt(input)");
    debug!("001");

    let cdp_hash = get_udt_hash(args, ID::CDPType as u32)?;
    let cdp_type = load_cell_type_hash(1, Source::Input)?;
    debug!("002");
    if cdp_type != Some(cdp_hash) {
        return Err(Error::ErrCDP);
    }

    let self_hash = load_cell_type_hash(0, Source::GroupOutput)?;
    let type_hash1 = load_cell_type_hash(1, Source::Output)?;
    debug!("003");
    if self_hash != type_hash1 {
        return Err(Error::ErrIndex);
    }

    Ok(())
}

fn new_auction(args: &Bytes) -> Result<(), Error> {
    load_cell_data(1, Source::GroupInput).expect_err("more than one debt(input)");
    load_cell_data(0, Source::GroupOutput).expect_err("exist debt(output)");
    let debt_data = load_cell_data(0, Source::GroupInput)?;
    let (udt_amount, _, id) = parse_data(debt_data.clone());
    let udt_hash = get_udt_hash(args, id)?;

    let type_hash = load_cell_type_hash(0, Source::Input)?;
    assert!(type_hash == Some(udt_hash), "error Input[0],not udt");
    let data = load_cell_data(0, Source::Input)?;
    assert!(data.len() >= UDT_VAL_LEN, "error udt data:{}", line!());
    let mut buf = [0u8; UDT_VAL_LEN];
    buf.copy_from_slice(&data[..UDT_VAL_LEN]);
    let v = u128::from_le_bytes(buf);
    assert!(v == udt_amount, "error input udt amount");

    let type_hash = load_cell_type_hash(0, Source::Output)?;
    assert!(type_hash == Some(udt_hash), "error output[0],not udt");
    let data = load_cell_data(0, Source::Output)?;
    assert!(data.len() >= UDT_VAL_LEN, "error udt data:{}", line!());
    let mut buf = [0u8; UDT_VAL_LEN];
    buf.copy_from_slice(&data[..UDT_VAL_LEN]);
    let v = u128::from_le_bytes(buf);
    assert!(v == udt_amount, "error output udt amount");

    let auction_hash = get_udt_hash(args, ID::AuctionType as u32)?;
    let auction_hash1 = load_cell_type_hash(1, Source::Output)?;
    assert!(
        auction_hash1 == Some(auction_hash),
        "error output[1],not auction"
    );

    let auction_data = load_cell_data(1, Source::Output)?;
    assert!(auction_data == debt_data, "error auction data");

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("debt script args is {:x?}", args);

    // return an error if args is invalid
    if args.is_empty() {
        return Err(Error::ErrArgs);
    }

    load_cell_data(1, Source::GroupOutput).expect_err("more than one debt(output)");
    match load_cell_data(0, Source::GroupOutput) {
        Ok(_) => {
            return liquidation(&args);
        }
        _ => {
            return new_auction(&args);
        }
    }

    // Ok(())
}
