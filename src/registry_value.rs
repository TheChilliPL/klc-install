#![allow(dead_code)]

use crate::registry_key::RegistryKey;
use crate::utils::ToU16Slice;
use widestring::U16CString;
use windows::Win32::System::Registry::*;

#[derive(Debug)]
pub struct RegistryValue<'a> {
    key: &'a RegistryKey,
    name: Option<String>,
    value: RegistryValueData,
}

impl RegistryValue<'_> {
    pub fn new(key: &RegistryKey, name: Option<String>, value: RegistryValueData) -> RegistryValue {
        RegistryValue { key, name, value }
    }

    pub fn new_from_data(
        key: &RegistryKey,
        name: Option<String>,
        type_code: REG_VALUE_TYPE,
        data: Vec<u8>,
    ) -> Result<RegistryValue, String> {
        let value = RegistryValueData::from_data(type_code, data)?;
        Ok(RegistryValue::new(key, name, value))
    }

    pub fn get_key(&self) -> &RegistryKey {
        self.key
    }

    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn get_value(&self) -> &RegistryValueData {
        &self.value
    }
}

#[derive(Debug)]
pub enum RegistryValueData {
    None,
    Binary(Vec<u8>),
    Dword(u32),
    Qword(u64),
    String(String),
    MultiString(Vec<String>),
    ExpandString(String),
}

impl RegistryValueData {
    pub fn from_data<'a>(
        type_code: REG_VALUE_TYPE,
        data: Vec<u8>,
    ) -> Result<RegistryValueData, String> {
        match type_code {
            REG_NONE => Ok(RegistryValueData::None),
            REG_BINARY => Ok(RegistryValueData::Binary(data)),
            REG_DWORD_LITTLE_ENDIAN => {
                if data.len() != 4 {
                    return Err("Invalid data length for REG_DWORD_LITTLE_ENDIAN!".to_string());
                }
                let dword = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                Ok(RegistryValueData::Dword(dword))
            }
            REG_DWORD_BIG_ENDIAN => {
                if data.len() != 4 {
                    return Err("Invalid data length for REG_DWORD_BIG_ENDIAN!".to_string());
                }
                let dword = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                Ok(RegistryValueData::Dword(dword))
            }
            REG_QWORD_LITTLE_ENDIAN => {
                if data.len() != 8 {
                    return Err("Invalid data length for REG_QWORD_LITTLE_ENDIAN!".to_string());
                }
                let qword = u64::from_le_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]);
                Ok(RegistryValueData::Qword(qword))
            }
            REG_SZ => {
                let string = U16CString::from_vec_truncate(data.to_u16_slice()).to_string();
                // if string.is_err() {
                //     return Err("Failed to parse UTF-16 data!".to_string());
                // }
                // let string = string.unwrap().to_string();
                if string.is_err() {
                    return Err("Failed to convert UTF-16 data to string!".to_string());
                }
                Ok(RegistryValueData::String(string.unwrap()))
            }
            REG_MULTI_SZ => {
                let data_16 = data.to_u16_slice();
                let mut strings = Vec::new();
                let mut i = 0;
                while i < data_16.len() {
                    let mut j = i;
                    while j < data_16.len() && data_16[j] != 0 {
                        // Might crash on last string?
                        j += 1;
                    }
                    let string = String::from_utf16_lossy(&data_16[i..j]);
                    strings.push(string);
                    i = j + 1;
                }
                Ok(RegistryValueData::MultiString(strings))
            }
            REG_EXPAND_SZ => {
                let string = String::from_utf16_lossy(data.to_u16_slice());
                Ok(RegistryValueData::ExpandString(string))
            }
            _ => Err(format!("Unsupported registry value type {}!", type_code.0)),
        }
    }

    pub fn to_raw(&self) -> (REG_VALUE_TYPE, Vec<u8>) {
        match self {
            RegistryValueData::None => (REG_NONE, Vec::new()),
            RegistryValueData::Binary(data) => (REG_BINARY, data.clone()),
            RegistryValueData::Dword(dword) => {
                (REG_DWORD_LITTLE_ENDIAN, dword.to_le_bytes().to_vec())
            }
            RegistryValueData::Qword(qword) => {
                (REG_QWORD_LITTLE_ENDIAN, qword.to_le_bytes().to_vec())
            }
            RegistryValueData::String(string) => {
                let mut data = Vec::new();
                for c in string.encode_utf16() {
                    data.push((c & 0xFF) as u8);
                    data.push((c >> 8) as u8);
                }
                data.push(0);
                data.push(0);
                (REG_SZ, data)
            }
            RegistryValueData::MultiString(strings) => {
                let mut data = Vec::new();
                for string in strings {
                    for c in string.encode_utf16() {
                        data.push((c & 0xFF) as u8);
                        data.push((c >> 8) as u8);
                    }
                    data.push(0);
                    data.push(0);
                }
                data.push(0);
                data.push(0);
                (REG_MULTI_SZ, data)
            }
            RegistryValueData::ExpandString(string) => {
                let mut data = Vec::new();
                for c in string.encode_utf16() {
                    data.push((c & 0xFF) as u8);
                    data.push((c >> 8) as u8);
                }
                data.push(0);
                data.push(0);
                (REG_EXPAND_SZ, data)
            }
        }
    }
}
