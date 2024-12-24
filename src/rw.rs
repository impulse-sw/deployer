use serde::{de::DeserializeOwned, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::OnceLock;
use std::path::{Path, PathBuf};
use crate::{CACHE_DIR, LOGS_DIR, PROJECT_CONF};

pub(crate) static VERBOSE: OnceLock<bool> = OnceLock::new();
const LOG_FILE_DELIMETER: &str = "================================================================";

pub(crate) fn read<T: DeserializeOwned + Default>(folder: impl AsRef<Path>, file: impl AsRef<Path>) -> T {
  let mut path = PathBuf::new();
  path.push(folder);
  path.push(file);
  
  match read_checked(path) {
    Err(e) => {
      log(format!("Error on file read: {:?}", e));
      Default::default()
    },
    Ok(v) => v,
  }
}

pub(crate) fn read_checked<T: DeserializeOwned>(filepath: impl AsRef<Path>) -> anyhow::Result<T> {
  let file = File::open(filepath.as_ref())?;
  let reader = BufReader::new(file);
  
  match filepath.as_ref().extension().unwrap().to_str().unwrap().to_lowercase().as_str() {
    "json" => Ok(serde_json::from_reader(reader)?),
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
    },
  }
}

pub(crate) fn copy_all(src: impl AsRef<Path>, dst: impl AsRef<Path>, ignore: &[&str]) -> anyhow::Result<()> {
  if src.as_ref().is_file() {
    if let Some(parent) = dst.as_ref().parent() {
      std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src.as_ref(), dst.as_ref())?;
    return Ok(())
  }
  std::fs::create_dir_all(&dst)?;
  
  for entry in std::fs::read_dir(src)? {
    let entry = entry?;
    let name = entry.file_name();
    let name = name.to_str().unwrap_or("");
    
    if ignore.contains(&name) { continue }
    
    log(format!("-> {}", name));
    
    let ty = entry.file_type()?;
    let d = dst.as_ref().join(entry.file_name());
    if ty.is_dir() {
      copy_all(entry.path(), d, ignore)?;
    } else if name == PROJECT_CONF {
      log(format!("Symlinking `{}` from {:?} to {:?}", name, entry.path(), d));
      symlink(std::fs::canonicalize(entry.path())?, d);
    } else if ty.is_file() {
      std::fs::copy(entry.path(), d)?;
    } else if ty.is_symlink() {
      symlink(std::fs::canonicalize(name)?, d);
    }
  }
  
  Ok(())
}

pub(crate) fn remove_all(path: impl AsRef<Path>) -> anyhow::Result<()> {
  if path.as_ref().is_file() {
    std::fs::remove_file(path)?;
  } else if path.as_ref().is_dir() {
    std::fs::remove_dir_all(path)?
  }
  
  Ok(())
}

pub(crate) fn symlink(src: impl AsRef<Path>, dst: impl AsRef<Path>) {
  use std::os::unix::fs::symlink as os_symlink;
  
  match os_symlink(src.as_ref(), dst) {
    Ok(_) => (),
    Err(e) => {
      log(format!("Skip `{}` due to: {:?}", src.as_ref().to_str().unwrap(), e));
    },
  }
}

pub(crate) fn log(s: impl AsRef<str>) {
  if *VERBOSE.wait() {
    println!("{}", s.as_ref());
  }
}

pub(crate) fn generate_build_log_filepath(
  project_name: &str,
  pipeline_short_name: &str,
  cache_dir: &Path,
) -> PathBuf {
  use chrono::Local;
  
  let mut logs_path = PathBuf::new();
  logs_path.push(cache_dir);
  logs_path.push(CACHE_DIR);
  logs_path.push(LOGS_DIR);
  if !logs_path.exists() { std::fs::create_dir_all(logs_path.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", logs_path)); }
  
  let curr_dt = Local::now();
  
  let log_path = logs_path.join(format!("{}-{}-{}.txt", project_name.replace('/', "-"), pipeline_short_name, curr_dt.format("%Y-%m-%d-%H:%M")));
  if log_path.exists() { build_log(&log_path, &[LOG_FILE_DELIMETER.to_string()]).expect("Current log file is unwriteable!"); }
  
  log_path
}

pub(crate) fn build_log(
  path: &Path,
  output: &[String],
) -> anyhow::Result<()> {
  use std::io::Write;
  
  let file = File::options().create(true).append(true).open(path)?;
  let mut writer = BufWriter::new(file);
  for line in output {
    let line = strip_ansi_escapes::strip(line.as_bytes());
    writer.write_all(&line)?;
    writer.write_all("\n".as_bytes())?;
  }
  
  Ok(())
}
