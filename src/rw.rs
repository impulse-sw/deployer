use serde::{de::DeserializeOwned, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::OnceLock;
use std::path::{Path, PathBuf};

pub(crate) static VERBOSE: OnceLock<bool> = OnceLock::new();

pub(crate) fn read<T: DeserializeOwned + Default>(folder: impl AsRef<Path>, file: impl AsRef<Path>) -> T {
  let mut path = PathBuf::new();
  path.push(folder);
  path.push(file);
  
  match read_checked(path) {
    Err(_) => T::default(),
    Ok(val) => val,
  }
}

pub(crate) fn read_checked<T: DeserializeOwned>(filepath: impl AsRef<Path>) -> anyhow::Result<T> {
  let file = File::open(filepath.as_ref())?;
  let reader = BufReader::new(file);
  
  match filepath.as_ref().extension().unwrap().to_str().unwrap().to_lowercase().as_str() {
    "json" => Ok(serde_json::from_reader(reader)?),
    "yaml" | "yml" => Ok(serde_yaml::from_reader(reader)?),
    _ => Err(anyhow::anyhow!("Unsupported file extension!"))
  }
}

pub(crate) fn write<T: Serialize>(folder: impl AsRef<Path>, file: impl AsRef<Path>, config: &T) {
  let mut path = PathBuf::new();
  path.push(folder);
  path.push(file.as_ref());
  let f = match File::create(path) {
    Ok(file) => file,
    Err(_) => {
      log(format!("Can't save `{:?}` config file!", file.as_ref().as_os_str()));
      return
    }
  };
  
  let writer = BufWriter::new(f);
  
  match serde_json::to_writer_pretty(writer, config) {
    Ok(_) => (),
    Err(_) => {
      log(format!("Can't save `{:?}` config file due to serialization error!", file.as_ref().as_os_str()));
      ()
    },
  }
}

pub(crate) fn copy_all(src: impl AsRef<Path>, dst: impl AsRef<Path>, ignore: &[&str]) {
  std::fs::create_dir_all(&dst).expect(format!("Can't create `{:?}` folder!", dst.as_ref()).as_str());
  
  if src.as_ref().is_file() {
    std::fs::copy(src.as_ref(), dst.as_ref().join(src.as_ref().file_name().unwrap())).unwrap();
    return
  }
  
  for entry in std::fs::read_dir(src).unwrap() {
    let entry = entry.unwrap();
    let name = entry.file_name();
    let name = name.to_str().unwrap_or("");
    
    if ignore.contains(&name) { continue }
    
    log(format!("-> {}", name));
    
    let ty = entry.file_type().unwrap();
    let d = dst.as_ref().join(entry.file_name());
    if ty.is_dir() {
      copy_all(entry.path(), d, ignore);
    } else if ty.is_file() {
      std::fs::copy(entry.path(), d).unwrap();
    } else if ty.is_symlink() {
      symlink(std::fs::canonicalize(name).expect("There is no such point symlink links to!"), d);
    }
  }
}

pub(crate) fn symlink(src: impl AsRef<Path>, dst: impl AsRef<Path>) {
  use std::os::unix::fs::symlink as os_symlink;
  
  match os_symlink(src.as_ref(), dst) {
    Ok(_) => (),
    Err(_) => {
      log(format!("Skip `{}`", src.as_ref().to_str().unwrap()));
      ()
    },
  }
}

pub(crate) fn log(s: impl AsRef<str>) {
  if *VERBOSE.wait() {
    println!("{}", s.as_ref());
  }
}
