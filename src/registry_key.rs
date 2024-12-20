#![allow(dead_code)]

use std::{
    fmt::{self, Display, Formatter},
    iter::from_fn,
    ptr::null_mut,
};

use widestring::U16CString;
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS, WIN32_ERROR},
        System::Registry::*,
    },
};

use crate::registry_value::{RegistryValue, RegistryValueData};

#[derive(Debug)]
pub struct RegistryKey {
    hkey: HKEY,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    NotFound,
    AccessDenied,
    Win32(WIN32_ERROR),
    Other(String),
}

impl From<WIN32_ERROR> for RegistryError {
    fn from(err: WIN32_ERROR) -> Self {
        match err {
            ERROR_FILE_NOT_FOUND => RegistryError::NotFound,
            ERROR_NO_MORE_ITEMS => RegistryError::NotFound,
            ERROR_ACCESS_DENIED => RegistryError::AccessDenied,
            _ => RegistryError::Win32(err),
        }
    }
}

impl Display for RegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::NotFound => write!(f, "Registry key or value not found!"),
            RegistryError::AccessDenied => write!(f, "Access denied!"),
            RegistryError::Win32(e) => write!(f, "Win32 error: {}", e.0),
            RegistryError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        if self.is_root_key() {
            return;
        }
        _ = unsafe { RegCloseKey(self.hkey) };
        // if err.is_err() {
        //     panic!("Failed to close registry key! Error code: {}", err.0);
        // }
    }
}

impl PartialEq for RegistryKey {
    fn eq(&self, other: &Self) -> bool {
        if self.hkey == other.hkey {
            return true;
        }
        self.path.eq_ignore_ascii_case(&other.path)
    }
}
impl Eq for RegistryKey {}

impl RegistryKey {
    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn get_name(&self) -> &str {
        self.path.split("\\").last().unwrap()
    }

    pub fn is_root_key(&self) -> bool {
        !self.path.contains("\\")
    }

    pub fn get_subkey(&self, name: &str) -> Result<RegistryKey, RegistryError> {
        let mut name = U16CString::from_str(name).map_err(|e| {
            RegistryError::Other(format!("Couldn't convert string to UTF16! {}", e))
        })?;
        let mut hkey = Default::default();
        let hkey_err = unsafe {
            RegOpenKeyExW(
                self.hkey,
                PWSTR(name.as_mut_ptr()),
                0,
                KEY_ALL_ACCESS,
                &mut hkey,
            )
        };
        if hkey_err.is_err() {
            return Err(RegistryError::from(hkey_err));
        }
        let path: String = format!("{}\\{}", self.path, name.to_string().unwrap());
        Ok(RegistryKey { hkey, path })
    }

    pub fn get_parent(&self) -> Result<RegistryKey, RegistryError> {
        if self.is_root_key() {
            return Err(RegistryError::Other(
                "Root keys have no parents!".to_string(),
            ));
        }
        let path = self.path[..self.path.rfind("\\").ok_or(RegistryError::Other(format!(
            "Error getting parent of key {}",
            self.path
        )))?]
            .to_string();
        RegistryKey::from_path(&path)
    }

    pub fn create_subkey(&self, name: &str) -> Result<RegistryKey, RegistryError> {
        let mut name = U16CString::from_str(name).map_err(|e| {
            RegistryError::Other(format!("Couldn't convert string to UTF16! {}", e))
        })?;

        let mut hkey = HKEY::default();
        let hkey_err = unsafe {
            RegCreateKeyExW(
                self.hkey,
                PWSTR(name.as_mut_ptr()),
                0,
                None, // No user-defined class
                REG_OPTION_NON_VOLATILE,
                KEY_ALL_ACCESS,
                // REG_SAM_FLAGS::default(), // Default security access rights
                None, // Default security attributes
                &mut hkey,
                None, // No disposition information
            )
        };

        if hkey_err.is_err() {
            return Err(RegistryError::from(hkey_err));
        }

        let path: String = format!("{}\\{}", self.path, name.to_string().unwrap());

        Ok(RegistryKey { hkey, path })
    }

    pub fn get_value(&self, name: Option<&str>) -> Result<RegistryValue, RegistryError> {
        let mut name_str = if name == None {
            None
        } else {
            Some(
                U16CString::from_str(name.unwrap())
                    .map_err(|e| RegistryError::Other(format!("{}", e.to_string())))?,
            )
        };

        let mut value_type = Default::default();
        let mut value_len = 0;
        let value_err = unsafe {
            RegGetValueW(
                self.hkey,
                None,
                PWSTR(
                    name_str
                        .as_mut()
                        .map(|it| it.as_mut_ptr())
                        .unwrap_or(null_mut()),
                ),
                RRF_RT_ANY,
                Some(&mut value_type),
                None,
                Some(&mut value_len),
            )
        };

        if value_err.is_err() {
            return Err(RegistryError::from(value_err));
        }

        let mut value_buf = vec![0u8; value_len as usize];
        let value_err = unsafe {
            RegGetValueW(
                self.hkey,
                None,
                PWSTR(
                    name_str
                        .as_mut()
                        .map(|it| it.as_mut_ptr())
                        .unwrap_or(null_mut()),
                ),
                RRF_RT_ANY,
                Some(&mut value_type),
                Some(value_buf.as_mut_ptr() as *mut _),
                Some(&mut value_len),
            )
        };

        if value_err.is_err() {
            return Err(RegistryError::from(value_err));
        }

        let value =
            RegistryValue::new_from_data(self, name.map(|s| s.to_string()), value_type, value_buf);

        value.map_err(|e| RegistryError::Other(e))
    }

    pub fn try_get_value(
        &self,
        name: Option<&str>,
    ) -> Result<Option<RegistryValue>, RegistryError> {
        let res = self.get_value(name);
        if res.is_err() {
            let err = res.unwrap_err();
            if err == RegistryError::NotFound {
                return Ok(None);
            }
            return Err(err);
        }
        Ok(Some(res.unwrap()))
    }

    pub fn set_value(
        &self,
        name: Option<&str>,
        value: RegistryValueData,
    ) -> Result<(), RegistryError> {
        let mut name_str = if name == None {
            None
        } else {
            Some(
                U16CString::from_str(name.unwrap())
                    .map_err(|e| RegistryError::Other(format!("{}", e.to_string())))
                    .unwrap(),
            )
        };

        let (value_type, value_data) = value.to_raw();

        let value_err = unsafe {
            RegSetValueExW(
                self.hkey,
                PWSTR(
                    name_str
                        .as_mut()
                        .map(|it| it.as_mut_ptr())
                        .unwrap_or(null_mut()),
                ),
                0,
                value_type,
                Some(&value_data),
            )
        };

        if value_err.is_err() {
            return Err(RegistryError::from(value_err));
        }

        Ok(())
    }

    pub fn count_children(&self) -> Result<usize, RegistryError> {
        let mut children_count: u32 = 0;
        let info_err = unsafe {
            RegQueryInfoKeyW(
                self.hkey,
                PWSTR::null(),
                None,
                None,
                Some(&mut children_count),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        };

        if info_err.is_err() {
            return Err(RegistryError::from(info_err));
        }

        Ok(children_count as usize)
    }

    pub fn iter_children_names(
        &self,
    ) -> Box<dyn Iterator<Item = Result<String, RegistryError>> + '_> {
        // First we try getting the maximum length of the subkey names
        let mut max_name_len: u32 = 0;
        let info_err = unsafe {
            RegQueryInfoKeyW(
                self.hkey,
                PWSTR::null(),
                None,
                None,
                None,
                Some(&mut max_name_len), // Maximum length of subkey names, not including null terminator
                None,
                None,
                None,
                None,
                None,
                None,
            )
        };

        if info_err.is_err() {
            return Box::new([Err(RegistryError::from(info_err))].into_iter());
        }

        let mut name_buf = vec![0u16; max_name_len as usize + 1];
        let mut index = 0;

        return Box::new(from_fn(move || {
            let mut name_len = name_buf.len() as u32 + 1;
            let enum_err = unsafe {
                RegEnumKeyExW(
                    self.hkey,
                    index,
                    PWSTR(name_buf.as_mut_ptr()),
                    &mut name_len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                )
            };

            if enum_err.is_err() {
                if enum_err == ERROR_NO_MORE_ITEMS {
                    return None;
                }

                return Some(Err(RegistryError::from(enum_err)));
            }

            index += 1;

            let name = U16CString::from_vec(name_buf[..name_len as usize].to_vec());

            Some(
                name.map(|n| n.to_string().unwrap())
                    .map_err(|e| RegistryError::Other(e.to_string())),
            )
        }));
    }

    pub fn iter_children(
        &self,
    ) -> Box<dyn Iterator<Item = Result<RegistryKey, RegistryError>> + '_> {
        Box::new(
            self.iter_children_names()
                .map(move |name_res| name_res.and_then(|name| self.get_subkey(&name))),
        )
    }

    pub fn close(self) {
        drop(self)
    }

    pub fn local_machine() -> Self {
        Self {
            hkey: HKEY_LOCAL_MACHINE,
            path: "HKEY_LOCAL_MACHINE".to_string(),
        }
    }

    pub fn current_config() -> Self {
        Self {
            hkey: HKEY_CURRENT_CONFIG,
            path: "HKEY_CURRENT_CONFIG".to_string(),
        }
    }

    pub fn classes_root() -> Self {
        Self {
            hkey: HKEY_CLASSES_ROOT,
            path: "HKEY_CLASSES_ROOT".to_string(),
        }
    }

    pub fn current_user() -> Self {
        Self {
            hkey: HKEY_CURRENT_USER,
            path: "HKEY_CURRENT_USER".to_string(),
        }
    }

    pub fn users() -> Self {
        Self {
            hkey: HKEY_USERS,
            path: "HKEY_USERS".to_string(),
        }
    }

    fn get_root_from_name(name: &str) -> Result<Self, RegistryError> {
        match name.to_uppercase().as_str() {
            "HKEY_LOCAL_MACHINE" | "HKLM" => Ok(Self::local_machine()),
            "HKEY_CURRENT_CONFIG" | "HKCC" => Ok(Self::current_config()),
            "HKEY_CLASSES_ROOT" | "HKCR" => Ok(Self::classes_root()),
            "HKEY_CURRENT_USER" | "HKCU" => Ok(Self::current_user()),
            "HKEY_USERS" | "HKU" => Ok(Self::users()),
            _ => Err(RegistryError::Other(format!(
                "Invalid root key name: {}",
                name
            ))),
        }
    }

    pub fn from_path(path: &str) -> Result<Self, RegistryError> {
        let splitter = path.split_once("\\");

        if splitter.is_none() {
            return Self::get_root_from_name(path);
        }

        let (root_name, subkey_name) = splitter.unwrap();

        let root = Self::get_root_from_name(root_name)?;
        root.get_subkey(subkey_name)
    }
}

#[cfg(test)]
mod test {
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;

    use crate::registry_value::RegistryValueData;

    use super::RegistryKey;

    #[test]
    fn test_registry_keys() {
        use crate::registry_key::RegistryKey;

        let local_machine_key = RegistryKey::local_machine();
        assert_eq!(local_machine_key.get_path(), "HKEY_LOCAL_MACHINE");
        assert_eq!(local_machine_key.get_name(), "HKEY_LOCAL_MACHINE");
        assert_eq!(local_machine_key.hkey, HKEY_LOCAL_MACHINE);
        assert!(local_machine_key.is_root_key());

        let system_key = local_machine_key.get_subkey("SYSTEM").unwrap();
        assert_eq!(system_key.get_path(), "HKEY_LOCAL_MACHINE\\SYSTEM");
        assert_eq!(system_key.get_name(), "SYSTEM");
        assert!(!system_key.is_root_key());

        let local_machine_key_2 = system_key.get_parent().unwrap();
        assert_eq!(local_machine_key_2, local_machine_key);

        let ccs = system_key.get_subkey("CurrentControlSet").unwrap();
        assert_eq!(
            ccs.get_path(),
            "HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet"
        );
        assert_eq!(ccs.get_name(), "CurrentControlSet");
        assert!(!ccs.is_root_key());

        let system_key_2 = ccs.get_parent().unwrap();
        assert_eq!(system_key_2, system_key);
    }

    #[test]
    fn test_registry_value_read() {
        let key = RegistryKey::from_path("HKCU\\Volatile Environment").unwrap();

        let value = key.get_value(Some("USERNAME")).unwrap();

        assert_eq!(value.get_key(), &key);
        assert_eq!(value.get_name(), Some("USERNAME"));

        match value.get_value() {
            RegistryValueData::String(s) => {
                println!("Username is: {}", s);
            }
            _ => panic!("Expected string value!"),
        }
    }
}
