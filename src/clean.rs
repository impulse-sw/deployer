use std::path::PathBuf;

use crate::{DEPLOY_CACHE_SUBDIR, DEPLOY_ARTIFACTS_SUBDIR};

pub(crate) fn clean(
  cache_dir: &str,
  include_this: bool,
) {
  let mut path = PathBuf::new();
  path.push(cache_dir);
  path.push(DEPLOY_CACHE_SUBDIR);
  
  let _ = std::fs::remove_dir_all(path);
  
  if include_this {
    let curr_dir = std::env::current_dir().expect("Can't get current dir!");
    let artifacts_dir = curr_dir.join(DEPLOY_ARTIFACTS_SUBDIR);
    if artifacts_dir.as_path().exists() {
      let _ = std::fs::remove_dir_all(artifacts_dir);
    }
  }
}
