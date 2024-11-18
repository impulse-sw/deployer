use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

use crate::{DEPLOY_CACHE_SUBDIR, DEPLOY_ARTIFACTS_SUBDIR};
use crate::cmd::BuildArgs;
use crate::configs::DeployerProjectOptions;
use crate::rw::{copy_all, symlink, log};

pub(crate) fn build(
  config: &mut DeployerProjectOptions,
  cache_dir: &str,
  args: &mut BuildArgs,
) {
  let mut build_path = PathBuf::new();
  build_path.push(cache_dir);
  build_path.push(DEPLOY_CACHE_SUBDIR);
  std::fs::create_dir_all(build_path.as_path()).expect(format!("Can't create `{:?}` folder!", build_path).as_str());
  
  let curr_dir = std::env::current_dir().expect("Can't get current dir!");
  let artifacts_dir = curr_dir.join(DEPLOY_ARTIFACTS_SUBDIR);
  std::fs::create_dir_all(artifacts_dir.as_path()).expect(format!("Can't create `{:?}` folder!", artifacts_dir).as_str());
  
  if config.last_build.is_none() { args.fresh = true; }
  
  let uuid = match args.fresh {
    true => {
      let uuid = format!("deploy-build-{}", Uuid::new_v4().to_string());
      config.builds.push(uuid.clone());
      config.last_build = Some(uuid.clone());
      uuid
    },
    false => {
      config.last_build.as_ref().unwrap().to_owned()
    },
  };
  
  build_path.push(uuid.clone());
  
  log(format!("{:?}", build_path));
  
  copy_all(".", build_path.as_path(), &["Cargo.lock", "target", ".git", "deploy-config.json", DEPLOY_ARTIFACTS_SUBDIR, &uuid]);
  
  if args.with_cache {
    match args.copy_cache {
      false => {
        symlink(curr_dir.join("Cargo.lock"), build_path.join("Cargo.lock"));
        log("-> Cargo.lock");
        symlink(curr_dir.join("target"), build_path.join("target"));
        log("-> target/*");
      },
      true => {
        std::fs::copy(curr_dir.join("Cargo.lock"), build_path.join("Cargo.lock")).unwrap();
        log("-> Cargo.lock");
        copy_all(curr_dir.join("target"), build_path.as_path(), &[".git", "deploy-config.json", DEPLOY_ARTIFACTS_SUBDIR, &uuid]);
      },
    }
  }
  
  // if !config.build_command.is_empty() {
  //   Command::new("/bin/bash")
  //     .arg("-c")
  //     .arg(config.build_command.as_str())
  //     .spawn()
  //     .expect("Can't build the project!")
  //     .wait()
  //     .unwrap();
  // }
  
  println!("{}", build_path.to_str().expect("Can't convert `Path` to string!"));
  
//   for artifact in &config.artifacts {
//     let artifact_path = build_path.join(artifact);
//     if !std::fs::exists(artifact_path.clone()).expect("Can't check if provided artifacts exists!") {
//       panic!("There is no `{:?}` artifact!", artifact_path);
//     } else {
//       if artifact_path.as_path().is_dir() {
//         copy_all(artifact_path.as_path(), artifacts_dir.join(artifact_path.file_name().unwrap()), &[DEPLOY_ARTIFACTS_SUBDIR]);
//       } else if artifact_path.as_path().is_file() {
//         copy_all(artifact_path.as_path(), artifacts_dir.as_path(), &[DEPLOY_ARTIFACTS_SUBDIR]);
//       }
//       
//       println!("<- {}", artifact);
//     }
//   }
}
