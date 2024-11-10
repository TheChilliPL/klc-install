use std::{fs, io, path::Path};

pub fn move_file(from: &Path, to: &Path) -> Result<(), io::Error> {
    // First we check if the destination file already exists
    if to.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("File {} already exists.", to.to_str().unwrap()),
        ));
    }

    // We try renaming the file first
    let rename1 = fs::rename(from, to);
    if rename1.is_ok() {
        return Ok(());
    }

    // If renaming fails, we try copying the file and then deleting the original
    fs::copy(from, to)?;
    fs::remove_file(from)?;

    Ok(())
}
