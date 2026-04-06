use serde::{de::DeserializeOwned, Serialize};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

pub fn load_json_from_file<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
        Err(err) => return Err(err.to_string()),
    };

    let metadata = file.metadata().map_err(|err| err.to_string())?;
    if metadata.len() == 0 {
        return Ok(T::default());
    }

    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|err| err.to_string())
}

pub fn save_json_to_file_atomic<T>(path: &Path, data: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let file_name = path
        .file_name()
        .and_then(|part| part.to_str())
        .ok_or_else(|| "invalid file name".to_string())?;
    let tmp_name = format!("{file_name}.tmp");
    let tmp_path: PathBuf = path.with_file_name(tmp_name);

    let mut file = File::create(&tmp_path).map_err(|err| err.to_string())?;
    let payload = serde_json::to_vec_pretty(data).map_err(|err| err.to_string())?;
    file.write_all(&payload).map_err(|err| err.to_string())?;
    file.flush().map_err(|err| err.to_string())?;
    file.sync_all().map_err(|err| err.to_string())?;

    std::fs::rename(&tmp_path, path).map_err(|err| err.to_string())?;

    #[cfg(not(windows))]
    if let Some(parent) = path.parent() {
        let dir_file = File::open(parent).map_err(|err| err.to_string())?;
        dir_file.sync_all().map_err(|err| err.to_string())?;
    }

    Ok(())
}
