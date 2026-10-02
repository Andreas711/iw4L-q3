use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Pk3Error {
    Io(std::io::Error),
    Zip(zip::result::ZipError),
    Missing(String),
}

impl core::fmt::Display for Pk3Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Zip(e) => write!(f, "{e}"),
            Self::Missing(name) => write!(f, "PK3 entry not found: {name}"),
        }
    }
}

impl std::error::Error for Pk3Error {}

impl From<std::io::Error> for Pk3Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<zip::result::ZipError> for Pk3Error {
    fn from(value: zip::result::ZipError) -> Self {
        Self::Zip(value)
    }
}

pub struct Pk3Archive {
    path: PathBuf,
    archive: zip::ZipArchive<File>,
    names: Vec<String>,
}

impl Pk3Archive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Pk3Error> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut names = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            names.push(archive.by_index(i)?.name().to_ascii_lowercase());
        }
        Ok(Self {
            path,
            archive,
            names,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read(&mut self, name: &str) -> Result<Vec<u8>, Pk3Error> {
        let wanted = name.replace('\\', "/").to_ascii_lowercase();
        let Some(index) = self.names.iter().position(|entry| entry == &wanted) else {
            return Err(Pk3Error::Missing(name.to_owned()));
        };
        let mut entry = self.archive.by_index(index)?;
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub fn contains(&self, name: &str) -> bool {
        let wanted = name.replace('\\', "/").to_ascii_lowercase();
        self.names.iter().any(|entry| entry == &wanted)
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }
}
