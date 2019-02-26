use crate::config_file::ConfigFile;
use std::{
    fs,
    io::prelude::*,
    path::{Path, PathBuf},
};

pub(super) struct CrateList {
    installed_crate_registry: Vec<String>,
    installed_crate_git: Vec<String>,
    old_crate_registry: Vec<String>,
    old_crate_git: Vec<String>,
    used_crate_registry: Vec<String>,
    used_crate_git: Vec<String>,
    orphan_crate_registry: Vec<String>,
    orphan_crate_git: Vec<String>,
}

impl CrateList {
    // create list of all types of crate present in directory
    pub(super) fn create_list(
        cache_dir: &Path,
        src_dir: &Path,
        checkout_dir: &Path,
        db_dir: &Path,
        config_file: &ConfigFile,
    ) -> Self {
        let installed_crate_registry = get_installed_crate_registry(src_dir, cache_dir);
        let installed_crate_git = get_installed_crate_git(checkout_dir, db_dir);

        let mut old_crate_registry = Vec::new();
        let mut version_removed_crate = remove_version(&installed_crate_registry);
        version_removed_crate.sort();
        for i in 0..(version_removed_crate.len() - 1) {
            if version_removed_crate[i] == version_removed_crate[i + 1] {
                let old_crate_name = installed_crate_registry[i].to_string();
                old_crate_registry.push(old_crate_name);
            }
        }
        old_crate_registry.sort();

        let mut old_crate_git = Vec::new();
        for crate_name in &installed_crate_git {
            if !crate_name.contains("-HEAD") {
                let name = crate_name.rsplitn(2, '-').collect::<Vec<&str>>();
                let mut full_name_list = Vec::new();
                for crates in fs::read_dir(db_dir).unwrap() {
                    let entry = crates.unwrap().path();
                    let path = entry.as_path();
                    let file_name = path.file_name().unwrap();
                    let file_name = file_name.to_str().unwrap().to_string();
                    if file_name.contains(name[1]) {
                        let output = std::process::Command::new("git")
                            .arg("log")
                            .arg("--pretty=format:'%h'")
                            .arg("--max-count=1")
                            .current_dir(path)
                            .output()
                            .unwrap();
                        let mut rev_value =
                            std::str::from_utf8(&output.stdout).unwrap().to_string();
                        rev_value.retain(|c| c != '\'');
                        let full_name = format!("{}-{}", name[1], rev_value);
                        full_name_list.push(full_name)
                    }
                }
                if !full_name_list.contains(crate_name) {
                    old_crate_git.push(crate_name.to_string());
                }
            }
        }
        old_crate_git.sort();

        let mut used_crate_registry = Vec::new();
        let mut used_crate_git = Vec::new();
        for path in &config_file.directory() {
            let list = list_cargo_lock(&Path::new(path));
            let (mut registry_crate, mut git_crate) = read_content(&list, db_dir);
            used_crate_registry.append(&mut registry_crate);
            used_crate_git.append(&mut git_crate);
        }

        let mut orphan_crate_registry = Vec::new();
        let mut orphan_crate_git = Vec::new();
        for crates in &installed_crate_registry {
            if !used_crate_registry.contains(crates) {
                orphan_crate_registry.push(crates.to_string());
            }
        }
        for crates in &installed_crate_git {
            if crates.contains("-HEAD") {
                let split_installed = crates.rsplitn(2, '-').collect::<Vec<&str>>();
                if used_crate_git.is_empty() {
                    orphan_crate_git.push(crates.to_string());
                }
                for used in &used_crate_git {
                    if !used.contains(split_installed[1]) {
                        orphan_crate_git.push(crates.to_string())
                    }
                }
            } else if !used_crate_git.contains(crates) {
                orphan_crate_git.push(crates.to_string());
            }
        }
        Self {
            installed_crate_registry,
            installed_crate_git,
            old_crate_registry,
            old_crate_git,
            used_crate_registry,
            used_crate_git,
            orphan_crate_registry,
            orphan_crate_git,
        }
    }

    pub(super) fn installed_registry(&self) -> Vec<String> {
        self.installed_crate_registry.to_owned()
    }

    pub(super) fn old_registry(&self) -> Vec<String> {
        self.old_crate_registry.to_owned()
    }

    pub(super) fn used_registry(&self) -> Vec<String> {
        self.used_crate_registry.to_owned()
    }

    pub(super) fn orphan_registry(&self) -> Vec<String> {
        self.orphan_crate_registry.to_owned()
    }

    pub(super) fn installed_git(&self) -> Vec<String> {
        self.installed_crate_git.to_owned()
    }

    pub(super) fn old_git(&self) -> Vec<String> {
        self.old_crate_git.to_owned()
    }

    pub(super) fn used_git(&self) -> Vec<String> {
        self.used_crate_git.to_owned()
    }

    pub(super) fn orphan_git(&self) -> Vec<String> {
        self.orphan_crate_git.to_owned()
    }
}

// remove version tag from crates full tag mini function of remove_version
// function
fn clear_version_value(a: &str) -> String {
    let list = a.rsplitn(2, '-');
    let mut value = String::new();
    for (i, val) in list.enumerate() {
        if i == 1 {
            value = val.to_string()
        }
    }
    value
}

// List out cargo.lock file present inside directory listed inside config file
fn list_cargo_lock(path: &Path) -> Vec<PathBuf> {
    let mut list = Vec::new();
    for entry in std::fs::read_dir(path).unwrap() {
        let data = entry.unwrap().path();
        let data = data.as_path();
        if data.is_dir() {
            let mut kids_list = list_cargo_lock(data);
            list.append(&mut kids_list);
        }
        if data.is_file() && data.ends_with("Cargo.lock") {
            list.push(data.to_path_buf());
        }
    }
    list
}

// Read out content of cargo.lock file to list out crates present so can be used
// for orphan clean
fn read_content(list: &[PathBuf], db_dir: &Path) -> (Vec<String>, Vec<String>) {
    let mut present_crate_registry = Vec::new();
    let mut present_crate_git = Vec::new();
    for lock_file in list.iter() {
        let lock_file = lock_file.to_str().unwrap();
        let mut buffer = String::new();
        let mut file = std::fs::File::open(lock_file).unwrap();
        file.read_to_string(&mut buffer).unwrap();
        let mut set_flag = 0;
        for line in buffer.lines() {
            if line.contains("[metadata]") {
                set_flag = 1;
                continue;
            }
            if set_flag == 1 {
                let mut split = line.split_whitespace();
                split.next();
                let name = split.next().unwrap();
                let version = split.next().unwrap();
                let source = split.next().unwrap();
                if source.contains("(registry+") {
                    let full_name = format!("{}-{}", name, version);
                    present_crate_registry.push(full_name);
                }
                if source.contains("git+") {
                    let mut path_db = db_dir.to_path_buf();
                    path_db.push(name);
                    if source.contains("?rev=") {
                        let rev: Vec<&str> = line.split("?rev=").collect();
                        let rev_sha: Vec<&str> = rev[1].split(')').collect();
                        let rev_value = rev_sha[1].to_string();
                        let rev_short_form = &rev_value[..=6];
                        let full_name = format!("{}-{}", name, rev_short_form);
                        present_crate_git.push(full_name);
                    } else if source.contains("?branch=") || source.contains("?tag=") {
                        let branch: Vec<&str> = if source.contains("?branch=") {
                            line.split("?branch=").collect()
                        } else {
                            line.split("?tag=").collect()
                        };
                        let branch: Vec<&str> = branch[1].split(')').collect();
                        let branch_value = branch[1];
                        let output = std::process::Command::new("git")
                            .arg("log")
                            .arg("--pretty=format:'%h'")
                            .arg("--max-count=1")
                            .arg(branch_value)
                            .current_dir(path_db)
                            .output()
                            .unwrap();
                        let rev_value = std::str::from_utf8(&output.stdout).unwrap();
                        let full_name = format!("{}-{}", name, rev_value);
                        present_crate_git.push(full_name);
                    } else {
                        let output = std::process::Command::new("git")
                            .arg("log")
                            .arg("--pretty=format:'%h'")
                            .arg("--max-count=1")
                            .current_dir(path_db)
                            .output()
                            .unwrap();
                        let rev_value = std::str::from_utf8(&output.stdout).unwrap();
                        let full_name = format!("{}-{}", name, rev_value);
                        present_crate_git.push(full_name);
                    }
                }
            }
        }
    }
    (present_crate_registry, present_crate_git)
}

// Function used to remove version from installed_crate_registry list so can be
// used for old clean flag
fn remove_version(installed_crate_registry: &[String]) -> Vec<String> {
    let mut removed_version = Vec::new();
    for i in installed_crate_registry.iter() {
        let data = clear_version_value(i);
        removed_version.push(data);
    }
    removed_version
}

fn get_installed_crate_registry(src_dir: &Path, cache_dir: &Path) -> Vec<String> {
    let mut installed_crate_registry = Vec::new();
    if src_dir.exists() {
        for entry in fs::read_dir(src_dir).unwrap() {
            let entry = entry.unwrap().path();
            let path = entry.as_path();
            let file_name = path.file_name().unwrap();
            let crate_name = file_name.to_str().unwrap().to_string();
            installed_crate_registry.push(crate_name)
        }
    }
    if cache_dir.exists() {
        for entry in fs::read_dir(cache_dir).unwrap() {
            let entry = entry.unwrap().path();
            let path = entry.as_path();
            let file_name = path.file_name().unwrap();
            let crate_name = file_name.to_str().unwrap().to_string();
            let splitted_name = crate_name.rsplitn(2, '.').collect::<Vec<&str>>();
            installed_crate_registry.push(splitted_name[1].to_owned());
        }
    }
    installed_crate_registry.sort();
    installed_crate_registry.dedup();
    installed_crate_registry
}

fn get_installed_crate_git(checkout_dir: &Path, db_dir: &Path) -> Vec<String> {
    let mut installed_crate_git = Vec::new();
    if checkout_dir.exists() {
        for entry in fs::read_dir(checkout_dir).unwrap() {
            let entry = entry.unwrap().path();
            let path = entry.as_path();
            let file_path = path.file_name().unwrap();
            for git_sha in fs::read_dir(path).unwrap() {
                let git_sha = git_sha.unwrap().path();
                let git_sha = git_sha.as_path();
                let git_sha = git_sha.file_name().unwrap();
                let git_sha = git_sha.to_str().unwrap().to_string();
                let file_name = file_path.to_str().unwrap().to_string();
                let splitted_name = file_name.rsplitn(2, '-').collect::<Vec<&str>>();
                let full_name = format!("{}-{}", splitted_name[1], git_sha);
                installed_crate_git.push(full_name)
            }
        }
    }
    if db_dir.exists() {
        for entry in fs::read_dir(db_dir).unwrap() {
            let entry = entry.unwrap().path();
            let path = entry.as_path();
            let file_name = path.file_name().unwrap();
            let file_name = file_name.to_str().unwrap().to_string();
            let full_name = format!("{}-HEAD", file_name);
            installed_crate_git.push(full_name);
        }
    }
    installed_crate_git.sort();
    installed_crate_git.dedup();
    installed_crate_git
}
