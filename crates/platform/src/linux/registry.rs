//! Registry no-op stub on Linux.

use crate::Result;

#[derive(Copy, Clone, Debug)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

pub fn read_dword(_hive: Hive, _subkey: &str, _value: &str) -> Option<u32> {
    None
}

pub fn read_string(_hive: Hive, _subkey: &str, _value: &str) -> Option<String> {
    None
}

pub fn write_string(_hive: Hive, _subkey: &str, _value: &str, _data: &str) -> Result<()> {
    Ok(())
}

pub fn delete_value(_hive: Hive, _subkey: &str, _value: &str) -> Result<()> {
    Ok(())
}
