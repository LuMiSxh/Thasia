use fs2::FileExt;
use std::{fs::File, path::Path};

pub struct InstanceGuard {
    _file: File,
}

impl InstanceGuard {
    pub fn acquire(data_dir: &Path) -> std::io::Result<Option<Self>> {
        let path = data_dir.join("thasia.lock");
        let file = File::create(path)?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { _file: file })),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error),
        }
    }
}
