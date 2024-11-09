use std::path::PathBuf;

use windows::{core::GUID, Win32::{System::Com::CoTaskMemFree, UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT}}};

pub fn get_known_folder(folderid: &GUID) -> Result<PathBuf, String> {
    let folder_pwstr = unsafe { SHGetKnownFolderPath(
        folderid,
        KF_FLAG_DEFAULT,
        None,
    ) }.map_err(|e| e.to_string())?;

    let folder_str = unsafe { folder_pwstr.to_string().map_err(|e| e.to_string())? };

    let folder = PathBuf::from(folder_str);

    unsafe { CoTaskMemFree(Some(folder_pwstr.as_ptr().cast())) };

    Ok(folder)
}
