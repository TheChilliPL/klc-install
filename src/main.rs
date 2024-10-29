use is_elevated::is_elevated;
mod registry_key;
mod registry_value;
mod utils;
use registry_key::RegistryKey;
use registry_value::{RegistryValue, RegistryValueData};

fn main() {
    if !is_elevated() {
        println!("Please run this program as an administrator. This program requires administrative privileges to access the registry.");
        return;
    }

    let layouts_key =
        RegistryKey::from_path("HKLM\\SYSTEM\\CurrentControlSet\\Control\\Keyboard Layouts")
            .unwrap();

    println!(
        "Successfully opened the Keyboard Layouts registry key {}.",
        layouts_key.get_path()
    );

    let layout_keys_iter = layouts_key.iter_children();

    for layout_key in layout_keys_iter {
        if layout_key.is_err() {
            println!("Failed to open a child registry key.");
            continue;
        }
        let layout_key = layout_key.unwrap();
        let layout_id = layout_key.try_get_value(Some("Layout Id")).unwrap();

        if layout_id.is_none() {
            println!("The Layout Id registry value was not found.");
            continue;
        }

        let layout_id = layout_id.unwrap();

        match layout_id.get_value() {
            RegistryValueData::String(s) => {
                let layout_id_u16 = u16::from_str_radix(s.as_str(), 16).unwrap();
                println!("Layout Id: {} (decimal {})", s, layout_id_u16);
            }
            _ => {
                println!("The Layout Id registry value is not a string.");
                continue;
            }
        }
    }

    // unsafe {
    //     let mut layouts_key = Default::default();
    //     let layouts_key_err = RegOpenKeyExW(HKEY_LOCAL_MACHINE, w!("SYSTEM\\CurrentControlSet\\Control\\Keyboard Layouts"), 0, KEY_READ, &mut layouts_key);
    //     // let a = RegistryKey::new(layouts_key);

    //     if layouts_key_err.is_err() {
    //         println!("Failed to open the Keyboard Layouts registry key. Error code: {}", layouts_key_err.0);
    //         return;
    //     }

    //     println!("Successfully opened the Keyboard Layouts registry key.");

    //     let mut key_entry_index = 0;
    //     let mut layout_ids_used = [false; 0xffff];
    //     let mut layout_id_max = 0;
    //     loop {
    //         let mut key_entry_name_len: u32 = 256;
    //         let mut key_entry_name_buf = vec![0u16; key_entry_name_len as usize];
    //         // let mut key_entry_name = U16CString::
    //         let key_enum_err = RegEnumKeyExW(layouts_key, key_entry_index, PWSTR(key_entry_name_buf.as_mut_ptr()), &mut key_entry_name_len, None, PWSTR::null(), None, None);
    //         key_entry_index += 1;

    //         if key_enum_err.is_err() {
    //             if key_enum_err == ERROR_NO_MORE_ITEMS {
    //                 break;
    //             }

    //             println!("Failed to enumerate the Keyboard Layouts registry key. Error code: {}", key_enum_err.0);
    //             return;
    //         }

    //         let mut key_entry_name = U16CString::from_vec_truncate(key_entry_name_buf);

    //         println!("Found entry: {}", key_entry_name.display());

    //         let mut key_entry = Default::default();
    //         let key_entry_err = RegOpenKeyExW(layouts_key, PWSTR(key_entry_name.as_mut_ptr()), 0, KEY_READ, &mut key_entry);

    //         if key_entry_err.is_err() {
    //             println!("Failed to open the Keyboard Layouts registry key entry. Error code: {}", key_entry_err.0);
    //             return;
    //         }

    //         let mut layout_id_len: u32 = 10;
    //         let mut layout_id_buf = vec![0u16; layout_id_len as usize];
    //         let layout_id_err = RegGetValueW(key_entry, None, w!("Layout Id"), RRF_RT_REG_SZ, None, Some(layout_id_buf.as_mut_ptr() as *mut _), Some(&mut layout_id_len));

    //         if layout_id_err.is_err() {
    //             if layout_id_err == ERROR_FILE_NOT_FOUND {
    //                 println!("The Layout Id registry value was not found.");
    //                 continue;
    //             }

    //             println!("Failed to read the Layout Id registry value. Error code: {}", layout_id_err.0);
    //             return;
    //         }

    //         let layout_id = U16CString::from_vec_truncate(layout_id_buf);
    //         let layout_id_u16 = u16::from_str_radix(layout_id.to_string().unwrap().as_str(), 16).unwrap();

    //         println!("Layout Id: {} (decimal {})", layout_id.display(), layout_id_u16);

    //         if layout_ids_used[layout_id_u16 as usize] {
    //             eprintln!("WARNING! Layout ID is a duplicate!")
    //         }
    //         layout_ids_used[layout_id_u16 as usize] = true;

    //         if layout_id_u16 > layout_id_max {
    //             layout_id_max = layout_id_u16
    //         }
    //     }
    //     println!("Finished enumerating the Keyboard Layouts registry key. Found {} entries.", key_entry_index);

    //     for (i, v) in layout_ids_used[..layout_id_max as usize].iter().enumerate() {
    //         println!("Layout ID {:04X} ({0} decimal) is {}!", i, if *v { "USED" } else { "UNUSED" });
    //     }
    // }
}
