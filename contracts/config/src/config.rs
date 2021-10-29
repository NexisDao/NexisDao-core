#[derive(Clone)]
pub enum ID {
    IncentiveParam = 1,
    BurnLock,
    UDTLock,
    CDPType,
    TAIType,
    ManagerTokenType,
    CommunityLock,
    DebtType,
    AuctionType,
    DebtSysType,
    ProposalType,
    AuctionLock,
    NervosDAOLock,
    NervosDAOType,
    DAOInfoType,
    DAOUDT,
    DebtLock,
}
// PledgeRate = n,  //UDT hash + interest + staking rate: hash+f32+f32
// UDTPrice = n+1,  //UDT Price + time: f32 + u64

pub const ID_LEN: usize = 4;

#[macro_export]
macro_rules! def_get_deps_data {
    () => {
        pub fn get_deps_data(dep: Bytes) -> Result<Vec<u8>, Error> {
            for i in 0.. {
                let type_hash = match ckb_std::high_level::load_cell_type_hash(i, Source::CellDep) {
                    Ok(data) => {
                        if let Some(d) = data {
                            d
                        } else {
                            [0u8; 32]
                        }
                    }
                    Err(err) => {
                        debug!("error index data, index:{}, error:{:?}", i, err);
                        return Err(err.into());
                    }
                };
                if dep[..] != type_hash[..] {
                    debug!("get_deps_data index:{}, hash:{:x?}", i, type_hash);
                    continue;
                }
                let value = match ckb_std::high_level::load_cell_data(i, Source::CellDep) {
                    Ok(data) => data,
                    Err(err) => {
                        debug!("error index data, index:{}, error:{:?}", i, err);
                        return Err(err.into());
                    }
                };
                debug!("get_deps_data index:{}, value:{:x?}", i, value);
                return Ok(value);
            }
            Err(ckb_std::error::SysError::IndexOutOfBound.into())
        }
    };
}

#[macro_export]
macro_rules! def_get_config_data {
    () => {
        fn get_config_data(config: Vec<u8>, index: u32) -> Result<Vec<u8>, Error> {
            for i in 0.. {
                let type_hash = match ckb_std::high_level::load_cell_type_hash(i, Source::CellDep) {
                    Ok(data) => {
                        if let Some(d) = data {
                            d
                        } else {
                            [0u8; 32]
                        }
                    }
                    Err(err) => {
                        debug!(
                            "error index data, index:{}, config id:{}, error:{:?}",
                            i, index, err
                        );
                        return Err(err.into());
                    }
                };
                if config[..] != type_hash[..] {
                    // debug!("get_config_data. index:{}, hash:{:x?}", i, type_hash);
                    continue;
                }
                let value = match ckb_std::high_level::load_cell_data(i, Source::CellDep) {
                    Ok(data) => data,
                    Err(err) => {
                        debug!("error index data, index:{}, error:{:?}", i, err);
                        return Err(err.into());
                    }
                };
                debug!(
                    "get_config_data. i:{}, value:{:x?}, index:{}",
                    i, value, index
                );
                if value.len() < 4 {
                    continue;
                }
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&value[..4]);
                let id = u32::from_le_bytes(buf);
                if id != index {
                    continue;
                }
                return Ok(value[4..].to_vec());
            }
            Err(ckb_std::error::SysError::IndexOutOfBound.into())
        }
    };
}

#[macro_export]
macro_rules! def_get_cell_time {
    () => {
        fn get_cell_time(so: Source, index: usize) -> Result<u64, Error> {
            let header = ckb_std::high_level::load_header(index, so)?.raw();
            let mut buf = [0u8; 8];
            buf.copy_from_slice(header.timestamp().as_slice());
            let time = u64::from_le_bytes(buf);

            return Ok(time);
        }
    };
}

#[macro_export]
macro_rules! def_get_time_of_config {
    () => {
        fn get_time_of_config(config: Vec<u8>, index: u32) -> Result<u64, Error> {
            for i in 0.. {
                let type_hash = match ckb_std::high_level::load_cell_type_hash(i, Source::CellDep) {
                    Ok(data) => {
                        if let Some(d) = data {
                            d
                        } else {
                            [0u8; 32]
                        }
                    }
                    Err(err) => {
                        debug!("error index data, index:{}, error:{:?}", i, err);
                        return Err(err.into());
                    }
                };
                if config[..] != type_hash[..] {
                    debug!("get_time_of_config. index:{}, hash:{:x?}", i, type_hash);
                    continue;
                }
                let value = match ckb_std::high_level::load_cell_data(i, Source::CellDep) {
                    Ok(data) => data,
                    Err(err) => {
                        debug!("error index data, index:{}, error:{:?}", i, err);
                        return Err(err.into());
                    }
                };
                if value.len() < 4 {
                    continue;
                }
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&value[..4]);
                let id = u32::from_le_bytes(buf);
                if id != index {
                    continue;
                }
                let header = ckb_std::high_level::load_header(i, Source::CellDep)?.raw();
                let mut buf = [0u8; 8];
                buf.copy_from_slice(header.timestamp().as_slice());
                let time = u64::from_le_bytes(buf);
                return Ok(time);
            }
            Err(ckb_std::error::SysError::IndexOutOfBound.into())
        }
    };
}

#[macro_export]
macro_rules! require {
    ($cond:expr) => {
        if !$cond {
            ckb_std::syscalls::debug(alloc::format!(
                "error line:{},{}",
                line!(),
                stringify!($cond)
            ));
            return Err(Error::from(line!()));
        }
    };
}
