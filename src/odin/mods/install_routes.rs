//! Where the files in an extracted Thunderstore package go under `BepInEx/`.
//!
//! Follows the install rules r2modman and Thunderstore publish for Valheim (see
//! `docs/adr/0001-thunderstore-install-routes.md`): each known top-level folder of the package
//! has its own `BepInEx` destination, and everything else lands in the mod's plugin folder.

use crate::errors::ValheimModError;
use fs_extra::dir::{self, CopyOptions};
use fs_extra::file;
use log::{debug, info};
use std::fs::{create_dir_all, read_dir};
use std::path::{Path, PathBuf};

/// How a route keeps one mod's files apart from another's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tracking {
  /// Installed into `<route>/<mod name>/`, recorded, and removed with the mod.
  Subdir,
  /// Merged straight into the route, never overwriting a file that is already there, and not
  /// recorded (the files belong to the server admin once they exist).
  MergeKeepExisting,
}

struct Route {
  /// Top-level folder name in the package, matched case-insensitively.
  folder: &'static str,
  /// Destination relative to the `BepInEx` directory.
  destination: &'static str,
  tracking: Tracking,
}

/// Valheim's r2modman install rules. `plugins` is handled separately: it is also where
/// anything unrouted goes, and where the package's `manifest.json` is kept.
const ROUTES: &[Route] = &[
  Route {
    folder: "patchers",
    destination: "patchers",
    tracking: Tracking::Subdir,
  },
  Route {
    folder: "core",
    destination: "core",
    tracking: Tracking::Subdir,
  },
  Route {
    folder: "monomod",
    destination: "monomod",
    tracking: Tracking::Subdir,
  },
  Route {
    folder: "SlimVML",
    destination: "SlimVML",
    tracking: Tracking::Subdir,
  },
  Route {
    folder: "config",
    destination: "config",
    tracking: Tracking::MergeKeepExisting,
  },
];

/// `MonoMod` patches are recognised by this suffix when they sit loose in the package root.
const MONOMOD_SUFFIX: &str = ".mm.dll";

fn move_error(e: impl std::fmt::Display) -> ValheimModError {
  ValheimModError::FileMoveError(e.to_string())
}

fn create_dir(path: &Path) -> Result<(), ValheimModError> {
  create_dir_all(path).map_err(|e| ValheimModError::DirectoryCreationError(e.to_string()))
}

/// Moves the contents of `src` into `dest`, overwriting what is there. `fs_extra` falls back
/// to copying when a rename would cross filesystems (the temp dir usually is on another one).
fn move_contents(src: &Path, dest: &Path) -> Result<(), ValheimModError> {
  create_dir(dest)?;
  let options = CopyOptions {
    overwrite: true,
    content_only: true,
    ..CopyOptions::new()
  };
  dir::move_dir(src, dest, &options).map_err(move_error)?;
  Ok(())
}

fn move_file(src: &Path, dest_dir: &Path) -> Result<PathBuf, ValheimModError> {
  create_dir(dest_dir)?;
  let dest = dest_dir.join(
    src
      .file_name()
      .ok_or_else(|| move_error("file has no name"))?,
  );
  let options = file::CopyOptions {
    overwrite: true,
    ..file::CopyOptions::new()
  };
  file::move_file(src, &dest, &options).map_err(move_error)?;
  Ok(dest)
}

/// Copies every file under `src` into `dest` that `dest` does not already have.
fn merge_keep_existing(src: &Path, dest: &Path) -> Result<(), ValheimModError> {
  for entry in walkdir::WalkDir::new(src).min_depth(1) {
    let entry = entry.map_err(move_error)?;
    let relative = entry.path().strip_prefix(src).map_err(move_error)?;
    let target = dest.join(relative);
    if entry.file_type().is_dir() {
      create_dir(&target)?;
    } else if target.exists() {
      info!("Keeping existing config {target:?}; not replacing it with the mod's copy");
    } else {
      if let Some(parent) = target.parent() {
        create_dir(parent)?;
      }
      std::fs::copy(entry.path(), &target).map_err(move_error)?;
    }
  }
  Ok(())
}

/// Records an installed path once, however many package entries landed in it.
fn record(installed: &mut Vec<PathBuf>, path: PathBuf) {
  if !installed.contains(&path) {
    installed.push(path);
  }
}

/// Installs an extracted (non-framework) package from `extracted` into `bepinex`, using
/// `mod_name` as each tracked route's subfolder. Returns the paths it installed, for cleanup
/// when the mod is later removed: the mod's plugin folder first, then any other tracked route
/// the package used. Config merged into `BepInEx/config` is deliberately not returned.
pub fn install_extracted_package(
  extracted: &Path,
  bepinex: &Path,
  mod_name: &str,
) -> Result<Vec<PathBuf>, ValheimModError> {
  let plugin_dir = bepinex.join("plugins").join(mod_name);
  create_dir(&plugin_dir)?;
  let mut installed = vec![plugin_dir.clone()];

  let entries: Vec<PathBuf> = read_dir(extracted)
    .map_err(move_error)?
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .collect();

  for path in &entries {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
      continue;
    };

    if path.is_dir() {
      if name.eq_ignore_ascii_case("plugins") {
        debug!("Routing {name}/ to {plugin_dir:?}");
        move_contents(path, &plugin_dir)?;
      } else if let Some(route) = ROUTES.iter().find(|r| r.folder.eq_ignore_ascii_case(name)) {
        match route.tracking {
          Tracking::Subdir => {
            let dest = bepinex.join(route.destination).join(mod_name);
            info!("Routing {name}/ to {dest:?}");
            move_contents(path, &dest)?;
            record(&mut installed, dest);
          }
          Tracking::MergeKeepExisting => {
            let dest = bepinex.join(route.destination);
            info!("Merging {name}/ into {dest:?}");
            merge_keep_existing(path, &dest)?;
          }
        }
      } else {
        // Not a known route: keep it inside the plugin folder, as before.
        move_contents(path, &plugin_dir.join(name))?;
      }
    } else if name.to_ascii_lowercase().ends_with(MONOMOD_SUFFIX) {
      let dest = bepinex.join("monomod").join(mod_name);
      info!("Routing {name} to {dest:?}");
      move_file(path, &dest)?;
      record(&mut installed, dest);
    } else {
      // manifest.json, README.md, icon.png, loose plugin DLLs.
      move_file(path, &plugin_dir)?;
    }
  }

  Ok(installed)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::tempdir;

  fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
  }

  fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"))
  }

  #[test]
  fn plugins_and_root_files_go_to_the_mod_plugin_folder() {
    let package = tempdir().unwrap();
    let bepinex = tempdir().unwrap();
    write(package.path(), "manifest.json", "{}");
    write(package.path(), "README.md", "readme");
    write(package.path(), "plugins/Mod.dll", "plugin");
    write(package.path(), "plugins/assets/bundle", "bundle");
    write(package.path(), "Loose.dll", "loose");

    let installed = install_extracted_package(package.path(), bepinex.path(), "Mod").unwrap();

    let plugin_dir = bepinex.path().join("plugins/Mod");
    assert_eq!(installed, vec![plugin_dir.clone()]);
    assert_eq!(read(plugin_dir.join("Mod.dll")), "plugin");
    assert_eq!(read(plugin_dir.join("assets/bundle")), "bundle");
    assert_eq!(read(plugin_dir.join("Loose.dll")), "loose");
    assert!(plugin_dir.join("manifest.json").exists());
    assert!(plugin_dir.join("README.md").exists());
  }

  /// The layout of ArgusMagnus-ServersideQoL, the package from discussion #1516.
  #[test]
  fn patchers_go_to_their_own_tracked_folder() {
    let package = tempdir().unwrap();
    let bepinex = tempdir().unwrap();
    write(package.path(), "manifest.json", "{}");
    write(
      package.path(),
      "patchers/ServersideQoL.Patchers.dll",
      "patcher",
    );
    write(package.path(), "plugins/ServersideQoL.dll", "plugin");
    write(package.path(), "plugins/ServersideQoL.deps.json", "deps");

    let installed =
      install_extracted_package(package.path(), bepinex.path(), "ServersideQoL").unwrap();

    let patcher_dir = bepinex.path().join("patchers/ServersideQoL");
    let plugin_dir = bepinex.path().join("plugins/ServersideQoL");
    assert_eq!(installed, vec![plugin_dir.clone(), patcher_dir.clone()]);
    assert_eq!(
      read(patcher_dir.join("ServersideQoL.Patchers.dll")),
      "patcher"
    );
    assert_eq!(read(plugin_dir.join("ServersideQoL.dll")), "plugin");
    assert!(
      !plugin_dir.join("patchers").exists(),
      "the patcher must not also be left under plugins, where BepInEx won't load it"
    );
  }

  #[test]
  fn core_monomod_and_slimvml_are_routed_and_folder_names_ignore_case() {
    let package = tempdir().unwrap();
    let bepinex = tempdir().unwrap();
    write(package.path(), "Plugins/Mod.dll", "plugin");
    write(package.path(), "Core/Lib.dll", "core");
    write(package.path(), "MonoMod/Assembly.mm.dll", "monomod");
    write(package.path(), "slimvml/Old.dll", "slim");
    write(package.path(), "Loose.MM.dll", "loose monomod");

    let installed = install_extracted_package(package.path(), bepinex.path(), "Mod").unwrap();

    let b = bepinex.path();
    assert_eq!(read(b.join("plugins/Mod/Mod.dll")), "plugin");
    assert_eq!(read(b.join("core/Mod/Lib.dll")), "core");
    assert_eq!(read(b.join("monomod/Mod/Assembly.mm.dll")), "monomod");
    assert_eq!(read(b.join("monomod/Mod/Loose.MM.dll")), "loose monomod");
    assert_eq!(read(b.join("SlimVML/Mod/Old.dll")), "slim");
    for dir in ["plugins/Mod", "core/Mod", "monomod/Mod", "SlimVML/Mod"] {
      assert_eq!(
        installed.iter().filter(|p| **p == b.join(dir)).count(),
        1,
        "{dir} should be recorded exactly once in {installed:?}"
      );
    }
  }

  #[test]
  fn config_is_merged_without_overwriting_and_is_not_tracked() {
    let package = tempdir().unwrap();
    let bepinex = tempdir().unwrap();
    write(package.path(), "config/mod.cfg", "mod default");
    write(package.path(), "config/sub/new.cfg", "new default");
    write(bepinex.path(), "config/mod.cfg", "admin edited");

    let installed = install_extracted_package(package.path(), bepinex.path(), "Mod").unwrap();

    let config = bepinex.path().join("config");
    assert_eq!(read(config.join("mod.cfg")), "admin edited");
    assert_eq!(read(config.join("sub/new.cfg")), "new default");
    assert!(
      installed.iter().all(|p| !p.starts_with(&config)),
      "config must never be recorded for cleanup: {installed:?}"
    );
  }

  #[test]
  fn unknown_folders_stay_inside_the_plugin_folder() {
    let package = tempdir().unwrap();
    let bepinex = tempdir().unwrap();
    write(package.path(), "Translations/en.json", "{}");

    install_extracted_package(package.path(), bepinex.path(), "Mod").unwrap();

    assert!(bepinex
      .path()
      .join("plugins/Mod/Translations/en.json")
      .exists());
  }

  #[test]
  fn reinstalling_replaces_tracked_files() {
    let bepinex = tempdir().unwrap();
    for version in ["v1", "v2"] {
      let package = tempdir().unwrap();
      write(package.path(), "patchers/P.dll", version);
      install_extracted_package(package.path(), bepinex.path(), "Mod").unwrap();
    }
    assert_eq!(read(bepinex.path().join("patchers/Mod/P.dll")), "v2");
  }
}
