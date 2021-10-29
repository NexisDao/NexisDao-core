// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;
use ckb_std::error::SysError;
// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{
        load_cell_data, load_cell_lock_hash, load_cell_type_hash, load_input_out_point,
        load_script, QueryIter,
    },
};

use crate::error::Error;
use config::{config::ID, def_get_cell_time, def_get_config_data, require};
def_get_config_data!();
def_get_cell_time!();

// u128 is 16 bytes
const UDT_VAL_LEN: usize = 16;
const SCRIPT_LEN: usize = 32;
const UDT_ID_LEN: usize = 4;
const MAX_AUCTION_TIME: u64 = 24 * 3600 * 1000;
const PUBLICITY_TIME: u64 = 24 * 3600 * 1000;
const TIME_LEN: usize = 8;
const OWNER_LEN: usize = 32;

/*
parse data to UDT.amount, TAI.amount, UDT.hash, owner.hash, time
    1.TAI.amount, u128, 16bit
    2.UDT.amount, u128, 16bit
    3.UDT config id, u32, 4bit
    4.owner(input[0].lock_hash): 32bit
*/
fn parse_data(data: Vec<u8>) -> Result<(u128, u128, u32, Vec<u8>), Error> {
    require!(data.len() >= (UDT_VAL_LEN + UDT_VAL_LEN + UDT_ID_LEN + OWNER_LEN));

    let mut start: usize = 0;
    let mut end = UDT_VAL_LEN;
    let tai_amount_buf = &data[start..end];
    start = end;
    end = start + UDT_VAL_LEN;
    let udt_amount_buf = &data[start..end];
    start = end;
    end = start + UDT_ID_LEN;
    let udt_id_buf = &data[start..end];
    start = end;
    end = start + OWNER_LEN;
    let owner_buf = &data[start..end];

    let mut buf_id = [0u8; UDT_ID_LEN];
    buf_id.copy_from_slice(udt_id_buf);
    let id = u32::from_le_bytes(buf_id);

    let mut buf = [0u8; UDT_VAL_LEN];
    buf.copy_from_slice(udt_amount_buf);
    let udt_amount = u128::from_le_bytes(buf);

    buf.copy_from_slice(tai_amount_buf);
    let tai_amount = u128::from_le_bytes(buf);
    let owner_hash = owner_buf.to_vec();

    Ok((udt_amount, tai_amount, id, owner_hash))
}

fn get_config_hash(conf: &Bytes, id: u32) -> Result<[u8; SCRIPT_LEN], Error> {
    let udt_data = get_config_data(conf.to_vec(), id)?;
    let mut udt_hash_buf = [0u8; SCRIPT_LEN];
    udt_hash_buf.copy_from_slice(&udt_data[..SCRIPT_LEN]);

    Ok(udt_hash_buf)
}

fn get_udt_amount(so: Source, udt_hash: [u8; SCRIPT_LEN], index: usize) -> Result<u128, Error> {
    let mut buf = [0u8; UDT_VAL_LEN];
    debug!("get_udt_amount, udt:{:x?}", udt_hash);

    let cell_type = load_cell_type_hash(index, so)?;
    require!(Some(udt_hash) == cell_type);

    let data = load_cell_data(index, so)?;
    debug!("index:{}, data:{:x?}", index, data);
    require!(data.len() >= UDT_VAL_LEN);
    buf.copy_from_slice(&data[..UDT_VAL_LEN]);
    let value = u128::from_le_bytes(buf);
    debug!("index:{}, data:{:?}, value:{}", index, data, value);
    Ok(value)
}

fn get_time(conf: &Bytes, id: u32) -> Result<u64, Error> {
    let mut buf_time = [0u8; TIME_LEN];
    debug!("010");
    let udt_price_buf = get_config_data(conf.to_vec(), id)?;
    require!(udt_price_buf.len() >= UDT_ID_LEN + TIME_LEN);
    buf_time.copy_from_slice(&udt_price_buf[UDT_ID_LEN..]);
    let time = u64::from_le_bytes(buf_time);
    Ok(time)
}

/*
Dutch auction: Starting from 2 times TAI, gradually decrease, and it will be 0 after 24 hours, first come first served
1. The number of UDT in the input is the same as the number of auctions
2. Enough TAI
*/
fn do_auction(args: &Bytes, data: Vec<u8>) -> Result<(), Error> {
    load_cell_data(0, Source::GroupOutput).expect_err("exist auction(output)");
    // same tx
    let outpoint1 = load_input_out_point(0, Source::Input).unwrap();
    let outpoint2 = load_input_out_point(1, Source::Input).unwrap();
    let tx1 = outpoint1.tx_hash().unpack();
    let tx2 = outpoint2.tx_hash().unpack();
    require!(tx1 == tx2);

    let (udt_amount, tai_amount, id, owner) = parse_data(data)?;
    let udt_hash = get_config_hash(args, id)?;
    let udt_amount1 = get_udt_amount(Source::Input, udt_hash, 0)?;
    require!(udt_amount1 == udt_amount);

    let cell_lock = get_config_hash(args, ID::AuctionLock as u32)?;
    let mut count = 0;
    QueryIter::new(load_cell_lock_hash, Source::Input).for_each(|lock| {
        if lock == cell_lock {
            count = count + 1;
        } else {
            assert_eq!(count, 2);
        }
    });
    require!(count == 2);

    // get time
    let start = get_cell_time(Source::GroupInput, 0)?;
    let end = get_time(args, id + 1)?;
    require!(end >= start + PUBLICITY_TIME);
    let mut t = end - start - PUBLICITY_TIME;

    let burn_lock = get_config_hash(args, ID::BurnLock as u32)?;
    let lock0 = load_cell_lock_hash(0, Source::Output)?;
    require!(burn_lock == lock0);

    let tai_hash = get_config_hash(args, ID::TAIType as u32)?;
    let mut burned: u128 = 0;
    for i in 0.. {
        match load_cell_lock_hash(i, Source::Input) {
            Ok(lock0) => {
                if lock0 != burn_lock{
                    continue;
                }
                let t = load_cell_type_hash(i,Source::Input)?.unwrap();
                require!(&t[..] == &tai_hash[..]);
                let udt_data = load_cell_data(i, Source::Input)?;
                let mut buf = [0u8; UDT_VAL_LEN];
                buf.copy_from_slice(&udt_data[..UDT_VAL_LEN]);
                burned = burned + u128::from_le_bytes(buf);
            },
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
    }
    // output0
    let burn_tai = get_udt_amount(Source::Output, tai_hash, 0)? - burned;
    require!(burn_tai > 0);

    // The auction price is lower than the loaned TAI, generating system debt
    if burn_tai < tai_amount {
        require!(t > MAX_AUCTION_TIME / 2);
        let debt_hash = get_config_hash(args, ID::DebtSysType as u32)?;
        let debt_hash1 = load_cell_type_hash(1, Source::Output)?;
        require!(debt_hash1 == Some(debt_hash));

        let data = load_cell_data(1, Source::Output)?;
        require!(data.len() >= UDT_VAL_LEN);
        let mut buf = [0u8; UDT_VAL_LEN];
        buf.copy_from_slice(&data[..UDT_VAL_LEN]);
        let v = u128::from_le_bytes(buf);
        require!(v == tai_amount - burn_tai);

        let tai_req =
            20000 * (MAX_AUCTION_TIME - t) as u128 / MAX_AUCTION_TIME as u128 * tai_amount / 10000;
        require!(burn_tai >= tai_req);
        return Ok(());
    }
    require!(burn_tai == tai_amount);
    // output1
    let lock = get_config_hash(args, ID::CommunityLock as u32)?;
    let lock1 = load_cell_lock_hash(1, Source::Output)?;
    require!(lock == lock1);

    let mut input_fee: u128 = 0;
    for i in 0.. {
        match load_cell_lock_hash(i, Source::Input) {
            Ok(lock0) => {
                if lock0 != lock{
                    continue;
                }
                let t = load_cell_type_hash(i,Source::Input)?.unwrap();
                require!(&t[..] == &tai_hash[..]);
                let udt_data = load_cell_data(i, Source::Input)?;
                let mut buf = [0u8; UDT_VAL_LEN];
                buf.copy_from_slice(&udt_data[..UDT_VAL_LEN]);
                input_fee = input_fee + u128::from_le_bytes(buf);
            },
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
    }

    let mut fee_tai = get_udt_amount(Source::Output, tai_hash, 1)? - input_fee;
    require!(fee_tai <= tai_amount / 10);
    if fee_tai == tai_amount / 10 {
        let lock2 = load_cell_lock_hash(2, Source::Output)?;
        require!(owner == lock2);

        let refunse_tai = get_udt_amount(Source::Output, tai_hash, 2)?;
        fee_tai += refunse_tai;
    }

    if t > MAX_AUCTION_TIME{
        t = MAX_AUCTION_TIME;
    }
    let tai_req = 20000 * (MAX_AUCTION_TIME - t) as u128 / MAX_AUCTION_TIME as u128 * tai_amount / 10000;
    debug!(
        "burn:{:?},fee:{:?},tai_req:{:?},tai_amount:{:?},tai_amount/10:{:?},time:{:?}",
        burn_tai,
        fee_tai,
        tai_req,
        tai_amount,
        tai_amount / 10,
        t
    );
    require!(fee_tai + burn_tai >= tai_req);

    Ok(())
}

/*
Create an auction cell:
1. Only in output
2. exist the debt
3. output[1] is UDT
4. the output lock is correct
*/
fn new_auction(args: &Bytes) -> Result<(), Error> {
    load_cell_data(0, Source::GroupInput).expect_err("exist auction(input)");

    // same tx
    let outpoint1 = load_input_out_point(0, Source::Input).unwrap();
    let outpoint2 = load_input_out_point(1, Source::Input).unwrap();
    let tx1 = outpoint1.tx_hash().unpack();
    let tx2 = outpoint2.tx_hash().unpack();
    require!(tx1 == tx2);

    let lock = load_cell_lock_hash(0, Source::Input)?;
    for i in 1.. {
        let lock_hash = match load_cell_lock_hash(i, Source::Input) {
            Ok(data) => data,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
        if i == 1 {
            require!(lock[..] == lock_hash[..]);
        } else {
            require!(lock[..] != lock_hash[..]);
        }
    }

    let data = load_cell_data(0, Source::GroupOutput)?;
    let debt_data = load_cell_data(1, Source::Input)?;

    require!(&debt_data[..] == &data[..]);

    let (udt_amount, _1, id, _2) = parse_data(data)?;

    let udt_hash = get_config_hash(args, id)?;

    let type_hash = load_cell_type_hash(0, Source::Output)?;
    require!(type_hash == Some(udt_hash));
    let data = load_cell_data(0, Source::Output)?;
    require!(data.len() >= UDT_VAL_LEN);
    let mut buf = [0u8; UDT_VAL_LEN];
    buf.copy_from_slice(&data[..UDT_VAL_LEN]);
    let v = u128::from_le_bytes(buf);
    require!(v == udt_amount);

    let debt_hash = get_config_hash(args, ID::DebtType as u32)?;
    let debt_hash1 = load_cell_type_hash(1, Source::Input)?;
    require!(debt_hash1 == Some(debt_hash));

    let output_hash = load_cell_type_hash(1, Source::Output)?;
    let self_hash = load_cell_type_hash(0, Source::GroupOutput)?;
    require!(Some(self_hash) == Some(output_hash));

    let lock = get_config_hash(args, ID::AuctionLock as u32)?;
    let lock0 = load_cell_lock_hash(0, Source::Output)?;
    let lock1 = load_cell_lock_hash(1, Source::Output)?;
    require!(lock == lock0);
    require!(lock == lock1);

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:x?}", args);

    require!(!args.is_empty());

    /*
    1. Only output: the creation process
    2. Only input: the auction process
    */
    load_cell_data(1, Source::GroupInput).expect_err("more than one auction(input)");
    load_cell_data(1, Source::GroupOutput).expect_err("more than one auction(output)");
    match load_cell_data(0, Source::GroupInput) {
        Ok(data) => {
            return do_auction(&args, data);
        }
        _ => {
            return new_auction(&args);
        }
    }
}
