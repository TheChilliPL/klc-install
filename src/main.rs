use std::{
    env::current_dir,
    error::Error,
    path::{Path, PathBuf},
};

use clap::{Args, Parser, Subcommand};
use is_elevated::is_elevated;
mod registry_key;
mod registry_value;
mod utils;
use registry_key::{RegistryError, RegistryKey};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    // TODO /// Forces the program to run non-interactively.
    // #[clap(short, long)]
    // non_interactive: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Lists keyboard layouts installed
    List {
        /// Lists all keyboard layouts installed.
        /// If off, only lists custom keyboard layouts
        #[clap(short, long, group = "kind")]
        all: bool,
    },

    /// Installs a keyboard layout
    Install {
        /// Path to the keyboard layout file.
        ///
        /// Can be a .KLC file or a .DLL file.
        file: String,

        /// Path to MSKLC 1.4 directory.
        ///
        /// If the file is a .KLC file, MSKLC must be placed in %PATH% or provided here.
        #[clap(long)]
        msklc: Option<String>,

        /// Registry key to install the layout under.
        ///
        /// Must be an 8-digit hexadecimal number, where the last 4 digits signify the language code.
        /// By default, it starts at F000xxxx and increments by 1 for each layout.
        #[clap(short, long, visible_alias("key"), value_name = "KEY")]
        registry_key: Option<String>,

        /// ID of the layout to use.
        ///
        /// Must be a 4-digit hexadecimal number that is not already in use and
        /// is at most F000.
        /// Uses the highest available ID by default.
        #[clap(short, long)]
        id: Option<String>,

        /// Name of the layout to use.
        ///
        /// If not provided, the name is taken from the layout file or left empty.
        #[clap(short, long)]
        name: Option<String>,

        /// Add localized Display Name registry value.
        ///
        /// Will use the localized name in the layout file if available.
        ///
        /// By default, true if explicit name is not provided.
        #[clap(short, long, action = clap::ArgAction::Set, value_name = "BOOL")]
        localize_name: Option<bool>,
    },

    /// Tries to update the specific keyboard layout
    Update {
        /// Path to the keyboard layout file.
        ///
        /// Can be a .KLC file or a .DLL file.
        file: String,
    },

    /// Uninstalls the specific keyboard layout
    Uninstall {
        #[command(flatten)]
        layout: LayoutIdent,

        /// Force uninstallation of the layout. WARNING: This can uninstall system layouts.
        #[clap(short('F'), long)]
        force: bool,

        /// Remove the DLL file associated with the layout.
        #[clap(short('d'), long)]
        remove_dll: bool,
    },
}

#[derive(Args, Debug)]
#[group(required = true)]
struct LayoutIdent {
    /// Registry key of the layout to uninstall.
    #[arg(long, visible_alias("key"), value_name = "KEY")]
    registry_key: Option<String>,

    /// ID of the layout to uninstall.
    #[arg(long)]
    id: Option<String>,

    /// Name of the layout to uninstall.
    #[arg(long)]
    name: Option<String>,
}

fn get_layouts_key() -> Result<RegistryKey, RegistryError> {
    RegistryKey::from_path("HKLM\\SYSTEM\\CurrentControlSet\\Control\\Keyboard Layouts")
}

fn list_layouts(all: bool) {
    let layouts_key: Result<RegistryKey, RegistryError> = get_layouts_key();

    if layouts_key.is_err() {
        panic!(
            "Failed to open the Keyboard Layouts registry key. {}",
            layouts_key.unwrap_err()
        );
    }

    let layouts_key = layouts_key.unwrap();

    let layout_keys_iter = layouts_key.iter_children();

    println!(
        "{:>8} {:<4} {:<32} {:<32} {}",
        "Key", "ID", "Name", "Display Name", "File"
    );

    let mut skipped = 0;

    for layout_key_err in layout_keys_iter {
        if layout_key_err.is_err() {
            println!(
                "Failed to open a child registry key. {}",
                layout_key_err.unwrap_err()
            );
            continue;
        }

        let layout_key = layout_key_err.unwrap();
        let layout_key_name = layout_key.get_name();

        let layout_key_hex = u32::from_str_radix(layout_key_name, 16).unwrap();
        if !all && layout_key_hex < 0x00800000 {
            skipped += 1;
            continue;
        }

        let layout_id = layout_key
            .try_get_value(Some("Layout Id"))
            .unwrap()
            .map(|v| v.unwrap_str());
        let layout_name = layout_key
            .try_get_value(Some("Layout Text"))
            .unwrap()
            .map(|v| v.unwrap_str());
        let layout_display = layout_key
            .try_get_value(Some("Layout Display Name"))
            .unwrap()
            .map(|v| v.unwrap_str());
        let layout_file = layout_key
            .try_get_value(Some("Layout File"))
            .unwrap()
            .map(|v| v.unwrap_str());

        println!(
            "{:>8} {:<4} {:<32} {:<32} {}",
            layout_key_name,
            layout_id.unwrap_or_else(|| "-".to_string()),
            layout_name.unwrap_or_else(|| "UNKNOWN".to_string()),
            layout_display.unwrap_or_else(|| "-".to_string()),
            layout_file.unwrap_or_else(|| "???.DLL".to_string()),
        );
    }

    if skipped > 0 {
        println!(
            "Skipped {} system layouts. Use -a|--all to show all.",
            skipped
        );
    }
}

/// Checks if MSKLC is installed in the given directory.
///
/// Returns the path to KBDUTOOL if found.
fn get_kbdutool(msklc_dir: &Path) -> Result<PathBuf, String> {
    let msklc_path = msklc_dir.canonicalize().map_err(|e| e.to_string())?;
    let mut kbdutool_path = msklc_path.join("kbdutool.exe");

    if !kbdutool_path.exists() {
        kbdutool_path = msklc_path.join("bin/i386/kbdutool.exe");

        if !kbdutool_path.exists() {
            return Err("KBDUTOOL was not found in the MSKLC directory!".to_string());
        }
    }

    Ok(kbdutool_path)
}

/// Tries to find MSKLC's KBDUTOOL in the PATH.
fn find_kbdutool_in_path() -> Result<PathBuf, String> {
    let path_env = std::env::var("PATH").map_err(|e| e.to_string())?;
    let path_env = path_env.split(';');

    for path in path_env {
        let path = Path::new(path);

        // Check for MSKLC
        let msklc_path = path.join("MSKLC.exe");

        if !msklc_path.exists() {
            continue;
        }

        return find_kbdutool_in_path();
    }

    Err("MSKLC was not found in PATH. Please provide the path to MSKLC using --msklc.".to_string())
}

fn install_layout(
    file: String,
    msklc: Option<String>,
    _registry_key: Option<String>,
    _id: Option<String>,
    _name: Option<String>,
    _localize_name: Option<bool>,
) {
    let file_path = Path::new(&file).canonicalize().unwrap();

    // let is_dll = file_path.ends_with(".dll");
    // if !is_dll && !file_path.ends_with(".klc") {
    //     panic!("The file must be a .KLC or .DLL file.");
    // }
    let extension = file_path.extension().unwrap().to_ascii_lowercase();

    if extension != "klc" && extension != "dll" {
        panic!("The file must be a .KLC or .DLL file.");
    }

    if extension == "klc" {
        // We need to compile KLC file

        // 1. Try to find MSKLC
        let kbdutool_path = if let Some(msklc) = msklc {
            get_kbdutool(&Path::new(&msklc)).unwrap()
        } else {
            find_kbdutool_in_path().unwrap()
        };

        // 2. Compile the KLC file

        let kbdutool_output = std::process::Command::new(kbdutool_path)
            .arg("-wum")
            .arg(&file_path)
            .output()
            .unwrap();

        println!(
            "KBDUTOOL output: {}",
            String::from_utf8_lossy(&kbdutool_output.stdout)
        );

        if !kbdutool_output.status.success() {
            panic!(
                "Failed to compile the KLC file. {}",
                String::from_utf8_lossy(&kbdutool_output.stderr)
            );
        }

        // 3. Get the compiled DLL file

        // Doesn't work if file name isn't the same as layout name...
        // Gotta parse the KLC file to get the layout name

        let dll_path = current_dir()
            .unwrap()
            .join(file_path.file_name().unwrap())
            .with_extension("dll")
            .canonicalize()
            .unwrap();

        println!("The compiled DLL file is at: {}", dll_path.display());

        // if !dll_path.exists() {
        //     panic!("The compiled DLL file was not found.");
        // }
    }

    todo!("All good for now!");
}

fn update_layout(_file: String) {
    todo!();
}

fn uninstall_layout(_layout: LayoutIdent, _force: bool, _remove_dll: bool) {
    todo!();
}

fn main() {
    let args = Cli::parse();

    // println!("{:#?}", args);

    if !is_elevated() {
        println!("Please run this program as an administrator. This program requires administrative privileges to access the registry.");
        // TODO add a way to elevate the process
        return;
    }

    match args.command {
        Commands::List { all } => list_layouts(all),
        Commands::Install {
            file,
            msklc,
            registry_key,
            id,
            name,
            localize_name,
        } => install_layout(file, msklc, registry_key, id, name, localize_name),
        Commands::Update { file } => update_layout(file),
        Commands::Uninstall {
            layout,
            force,
            remove_dll,
        } => uninstall_layout(layout, force, remove_dll),
    }

    return;

    // let layouts_key =
    //     RegistryKey::from_path("HKLM\\SYSTEM\\CurrentControlSet\\Control\\Keyboard Layouts")
    //         .unwrap();

    // println!(
    //     "Successfully opened the Keyboard Layouts registry key {}.",
    //     layouts_key.get_path()
    // );

    // let layout_keys_iter = layouts_key.iter_children();

    // for layout_key in layout_keys_iter {
    //     if layout_key.is_err() {
    //         println!("Failed to open a child registry key.");
    //         continue;
    //     }
    //     let layout_key = layout_key.unwrap();
    //     let layout_id = layout_key.try_get_value(Some("Layout Id")).unwrap();

    //     if layout_id.is_none() {
    //         println!("The Layout Id registry value was not found.");
    //         continue;
    //     }

    //     let layout_id = layout_id.unwrap();

    //     match layout_id.get_value() {
    //         RegistryValueData::String(s) => {
    //             let layout_id_u16 = u16::from_str_radix(s.as_str(), 16).unwrap();
    //             println!("Layout Id: {} (decimal {})", s, layout_id_u16);
    //         }
    //         _ => {
    //             println!("The Layout Id registry value is not a string.");
    //             continue;
    //         }
    //     }
    // }

    // // unsafe {
    // //     let mut layouts_key = Default::default();
    // //     let layouts_key_err = RegOpenKeyExW(HKEY_LOCAL_MACHINE, w!("SYSTEM\\CurrentControlSet\\Control\\Keyboard Layouts"), 0, KEY_READ, &mut layouts_key);
    // //     // let a = RegistryKey::new(layouts_key);

    // //     if layouts_key_err.is_err() {
    // //         println!("Failed to open the Keyboard Layouts registry key. Error code: {}", layouts_key_err.0);
    // //         return;
    // //     }

    // //     println!("Successfully opened the Keyboard Layouts registry key.");

    // //     let mut key_entry_index = 0;
    // //     let mut layout_ids_used = [false; 0xffff];
    // //     let mut layout_id_max = 0;
    // //     loop {
    // //         let mut key_entry_name_len: u32 = 256;
    // //         let mut key_entry_name_buf = vec![0u16; key_entry_name_len as usize];
    // //         // let mut key_entry_name = U16CString::
    // //         let key_enum_err = RegEnumKeyExW(layouts_key, key_entry_index, PWSTR(key_entry_name_buf.as_mut_ptr()), &mut key_entry_name_len, None, PWSTR::null(), None, None);
    // //         key_entry_index += 1;

    // //         if key_enum_err.is_err() {
    // //             if key_enum_err == ERROR_NO_MORE_ITEMS {
    // //                 break;
    // //             }

    // //             println!("Failed to enumerate the Keyboard Layouts registry key. Error code: {}", key_enum_err.0);
    // //             return;
    // //         }

    // //         let mut key_entry_name = U16CString::from_vec_truncate(key_entry_name_buf);

    // //         println!("Found entry: {}", key_entry_name.display());

    // //         let mut key_entry = Default::default();
    // //         let key_entry_err = RegOpenKeyExW(layouts_key, PWSTR(key_entry_name.as_mut_ptr()), 0, KEY_READ, &mut key_entry);

    // //         if key_entry_err.is_err() {
    // //             println!("Failed to open the Keyboard Layouts registry key entry. Error code: {}", key_entry_err.0);
    // //             return;
    // //         }

    // //         let mut layout_id_len: u32 = 10;
    // //         let mut layout_id_buf = vec![0u16; layout_id_len as usize];
    // //         let layout_id_err = RegGetValueW(key_entry, None, w!("Layout Id"), RRF_RT_REG_SZ, None, Some(layout_id_buf.as_mut_ptr() as *mut _), Some(&mut layout_id_len));

    // //         if layout_id_err.is_err() {
    // //             if layout_id_err == ERROR_FILE_NOT_FOUND {
    // //                 println!("The Layout Id registry value was not found.");
    // //                 continue;
    // //             }

    // //             println!("Failed to read the Layout Id registry value. Error code: {}", layout_id_err.0);
    // //             return;
    // //         }

    // //         let layout_id = U16CString::from_vec_truncate(layout_id_buf);
    // //         let layout_id_u16 = u16::from_str_radix(layout_id.to_string().unwrap().as_str(), 16).unwrap();

    // //         println!("Layout Id: {} (decimal {})", layout_id.display(), layout_id_u16);

    // //         if layout_ids_used[layout_id_u16 as usize] {
    // //             eprintln!("WARNING! Layout ID is a duplicate!")
    // //         }
    // //         layout_ids_used[layout_id_u16 as usize] = true;

    // //         if layout_id_u16 > layout_id_max {
    // //             layout_id_max = layout_id_u16
    // //         }
    // //     }
    // //     println!("Finished enumerating the Keyboard Layouts registry key. Found {} entries.", key_entry_index);

    // //     for (i, v) in layout_ids_used[..layout_id_max as usize].iter().enumerate() {
    // //         println!("Layout ID {:04X} ({0} decimal) is {}!", i, if *v { "USED" } else { "UNUSED" });
    // //     }
    // // }
}
