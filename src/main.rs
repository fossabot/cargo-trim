#![warn(unreachable_pub, anonymous_parameters, bare_trait_objects)]
#![deny(unsafe_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

mod config_file;
mod crate_detail;
mod create_app;
mod dir_path;
mod git_dir;
mod list_crate;
mod registry_dir;
#[cfg(test)]
mod test;

use crate::{
    config_file::ConfigFile, crate_detail::CrateDetail, dir_path::DirPath, git_dir::GitDir,
    list_crate::CrateList, registry_dir::RegistryDir,
};
use clap::{ArgMatches, Shell};
use colored::*;
use fs_extra::dir::get_size;
use pretty_bytes::converter::convert;
use std::{collections::HashMap, fs, path::PathBuf, process::Command};

fn main() {
    // set all dir path
    let dir_path = DirPath::set_dir_path();
    let mut file = fs::File::open(dir_path.config_dir().to_str().unwrap())
        .expect("failed to open config dir folder");
    let app = create_app::app().get_matches();
    let app = app.subcommand_matches("trim").unwrap();
    generate_completions(app);
    let mut git_subcommand = &ArgMatches::new();
    let mut registry_subcommand = &ArgMatches::new();
    if app.is_present("git") {
        git_subcommand = app.subcommand_matches("git").unwrap();
    }
    if app.is_present("registry") {
        registry_subcommand = app.subcommand_matches("registry").unwrap();
    }

    let dry_run_app = app.is_present("dry run");
    let dry_run_git = git_subcommand.is_present("dry run");
    let dry_run_registry = registry_subcommand.is_present("dry run");

    // Perform all modification of config file flag and subcommand operation
    let config_file = config_file::modify_config_file(&mut file, app, &dir_path.config_dir());

    // Perform action of removing config file with -c flag
    clear_config(&app, &dir_path);

    // Query about config file information
    config_subcommand(&app, &config_file);

    // Force remove all crates without reading config file also remove index .cache
    // folder
    let force_remove_app = app.is_present("force remove");
    let force_remove_git = git_subcommand.is_present("force remove");
    let force_remove_registry = registry_subcommand.is_present("force remove");
    force_remove(
        &dir_path,
        (force_remove_app, force_remove_git, force_remove_registry),
        (dry_run_app, dry_run_git, dry_run_registry),
    );

    // Perform git compress to .cargo/index
    git_compress(
        &app,
        &dir_path.index_dir(),
        &dir_path.checkout_dir(),
        &dir_path.db_dir(),
    );

    // Perform light cleanup
    let light_cleanup_app = app.is_present("light cleanup");
    let light_cleanup_git = git_subcommand.is_present("light cleanup");
    let light_cleanup_registry = registry_subcommand.is_present("light_cleanup");
    light_cleanup(
        &dir_path.checkout_dir(),
        &dir_path.src_dir(),
        &dir_path.index_dir(),
        (light_cleanup_app, light_cleanup_git, light_cleanup_registry),
        (dry_run_app, dry_run_git, dry_run_registry),
    );

    // Wipe a certain folder all together
    wipe_directory(&app, &dir_path);

    // create new CrateDetail struct
    let mut crate_detail = CrateDetail::new();

    // List out crates
    let list_crate = list_crate::CrateList::create_list(&dir_path, &config_file, &mut crate_detail);

    // Get Location where registry crates and git crates are stored out by cargo
    let mut registry_crates_location = registry_dir::RegistryDir::new(
        &dir_path.cache_dir(),
        &dir_path.src_dir(),
        &dir_path.index_dir(),
        list_crate.installed_registry(),
        dry_run_app || dry_run_registry,
    );

    let git_crates_location = git_dir::GitDir::new(
        &dir_path.checkout_dir(),
        &dir_path.db_dir(),
        dry_run_app || dry_run_git,
    );

    // Perform action on list subcommand
    list_subcommand(app, &list_crate, &crate_detail);

    // Perform action on -o flag matches which remove all old crates
    let old_app = app.is_present("old clean");
    let old_registry = registry_subcommand.is_present("old clean");
    let old_git = git_subcommand.is_present("old clean");
    old_clean(
        &list_crate,
        (old_app, old_registry, old_git),
        &mut registry_crates_location,
        &git_crates_location,
        &crate_detail,
    );

    // Orphan clean a crates which is not present in directory stored in directory
    // value of config file on -x flag
    let orphan_app = app.is_present("orphan clean");
    let orphan_git = git_subcommand.is_present("orphan clean");
    let orphan_registry = registry_subcommand.is_present("orphan clean");
    orphan_clean(
        &list_crate,
        (orphan_app, orphan_git, orphan_registry),
        &mut registry_crates_location,
        &git_crates_location,
        &crate_detail,
    );

    // Perform action for -q flag
    let query_size_app = app.is_present("query size");
    let query_size_git = git_subcommand.is_present("query size");
    let query_size_registry = registry_subcommand.is_present("query size");
    query_size(
        &dir_path,
        (query_size_app, query_size_git, query_size_registry),
        &list_crate,
        &crate_detail,
    );

    // Remove all crates by following config file
    let all_app = app.is_present("all");
    let all_git = git_subcommand.is_present("all");
    let all_registry = registry_subcommand.is_present("all");
    remove_all(
        &list_crate,
        &config_file,
        &mut registry_crates_location,
        &git_crates_location,
        (all_app, all_git, all_registry),
        &crate_detail,
    );

    // Remove certain crate provided with -r flag
    remove_crate(
        &list_crate,
        (&app, &git_subcommand, &registry_subcommand),
        &mut registry_crates_location,
        &git_crates_location,
        &crate_detail,
    );

    // Show top crates
    top_crates(&app, &git_subcommand, &registry_subcommand, &crate_detail);

    let cargo_toml_location = list_crate.cargo_toml_location().location_path();
    update_cargo_toml(&app, cargo_toml_location);
}

// Generate out completions script for different shell
fn generate_completions(app: &ArgMatches) {
    if app.is_present("completions") {
        let shell_value = app
            .subcommand_matches("completions")
            .unwrap()
            .value_of("shell")
            .unwrap();
        match shell_value {
            "bash" => create_app::app().gen_completions_to(
                "cargo-trim",
                Shell::Bash,
                &mut std::io::stdout(),
            ),
            "fish" => create_app::app().gen_completions_to(
                "cargo-trim",
                Shell::Fish,
                &mut std::io::stdout(),
            ),
            "zsh" => create_app::app().gen_completions_to(
                "cargo-trim",
                Shell::Zsh,
                &mut std::io::stdout(),
            ),
            "powershell" => create_app::app().gen_completions_to(
                "cargo-trim",
                Shell::PowerShell,
                &mut std::io::stdout(),
            ),
            "elvish" => create_app::app().gen_completions_to(
                "cargo-trim",
                Shell::Elvish,
                &mut std::io::stdout(),
            ),
            _ => {}
        }
    }
}

// Clear config file data
fn clear_config(app: &ArgMatches, dir_path: &DirPath) {
    if app.is_present("clear config") {
        if app.is_present("dry run") {
            println!("{} Cleared config file", "Dry run:".yellow());
        } else {
            fs::remove_file(dir_path.config_dir().as_path()).expect("failed to delete config file");
            println!("Cleared config file");
        }
    }
}

// Git compress git files
fn git_compress(app: &ArgMatches, index_dir: &PathBuf, checkout_dir: &PathBuf, db_dir: &PathBuf) {
    if app.is_present("git compress") {
        let value = app.value_of("git compress").unwrap();
        if (value == "index" || value == "all") && index_dir.exists() {
            for entry in fs::read_dir(index_dir).expect("failed to read registry index folder") {
                let repo_path = entry.unwrap().path();
                let file_name = repo_path
                    .file_name()
                    .expect("Failed to get a file name / folder name");
                println!(
                    "{}",
                    format!("Compressing {} registry index", file_name.to_str().unwrap())
                        .bright_blue()
                );
                run_git_compress_commands(&repo_path);
            }
        }
        if value.contains("git") || value == "all" {
            if (value == "git" || value == "git-checkout") && checkout_dir.exists() {
                for entry in fs::read_dir(checkout_dir).expect("failed to read checkout directory")
                {
                    let repo_path = entry.unwrap().path();
                    for rev in fs::read_dir(repo_path)
                        .expect("failed to read checkout directory sub directory")
                    {
                        let rev_path = rev.unwrap().path();
                        println!("{}", "Compressing git checkout".bright_blue());
                        run_git_compress_commands(&rev_path)
                    }
                }
            }
            if (value == "git" || value == "git-db") && db_dir.exists() {
                for entry in fs::read_dir(db_dir).expect("failed to read db dir") {
                    let repo_path = entry.unwrap().path();
                    println!("{}", "Compressing git db".bright_blue());
                    run_git_compress_commands(&repo_path);
                }
            }
        }
        println!("{}", "Git compress task completed".bright_blue());
    }
}

// run combination of commands which git compress a index of registry
fn run_git_compress_commands(repo_path: &PathBuf) {
    // Remove history of all checkout which will help in remove dangling commits
    if let Err(e) = Command::new("git")
        .arg("reflog")
        .arg("expire")
        .arg("--expire=now")
        .arg("--all")
        .current_dir(repo_path)
        .output()
    {
        panic!(format!("git reflog failed to execute due to error {}", e));
    } else {
        println!("{:70}.......Step 1/3", "  \u{251c} Completed git reflog");
    }

    // pack refs of branches/tags etc into one file know as pack-refs file for
    // effective repo access
    if let Err(e) = Command::new("git")
        .arg("pack-refs")
        .arg("--all")
        .arg("--prune")
        .current_dir(repo_path)
        .output()
    {
        panic!(format!(
            "git pack-refs failed to execute due to error {}",
            e
        ));
    } else {
        println!(
            "{:70}.......Step 2/3",
            "  \u{251c} Packed refs and tags successfully"
        );
    }

    // cleanup unnecessary file and optimize a local repo
    if let Err(e) = Command::new("git")
        .arg("gc")
        .arg("--aggressive")
        .arg("--prune=now")
        .current_dir(repo_path)
        .output()
    {
        panic!(format!("git gc failed to execute due to error {}", e));
    } else {
        println!(
            "{:70}.......Step 3/3",
            "  \u{2514} Cleaned up unnecessary files and optimize a files"
        );
    }
}

// light cleanup registry directory
fn light_cleanup(
    checkout_dir: &PathBuf,
    src_dir: &PathBuf,
    index_dir: &PathBuf,
    (light_cleanup_app, light_cleanup_git, light_cleanup_registry): (bool, bool, bool),
    (dry_run_app, dry_run_git, dry_run_registry): (bool, bool, bool),
) {
    if light_cleanup_app || light_cleanup_git || light_cleanup_registry {
        if light_cleanup_app || light_cleanup_registry {
            let dry_run = dry_run_app || dry_run_registry;
            delete_folder(checkout_dir, dry_run);
            // Delete out .cache folder also
            for entry in fs::read_dir(index_dir).expect("failed to read out index directory") {
                let entry = entry.unwrap().path();
                let registry_dir = entry.as_path();
                for folder in
                    fs::read_dir(registry_dir).expect("failed to read out registry directory")
                {
                    let folder = folder.unwrap().path();
                    let folder_name = folder
                        .file_name()
                        .expect("failed to get file name form registry directory sub folder");
                    if folder_name == ".cache" {
                        delete_folder(&folder, dry_run);
                    }
                }
            }
        }
        if light_cleanup_app || light_cleanup_git {
            let dry_run = dry_run_app || dry_run_git;
            delete_folder(src_dir, dry_run);
        }
    }
}

// Perform different operation for a list subcommand
fn list_subcommand(app: &ArgMatches, list_crate: &CrateList, crate_detail: &CrateDetail) {
    if app.is_present("list") {
        let list_subcommand = app.subcommand_matches("list").unwrap();
        if list_subcommand.is_present("old") {
            list_crate_type(
                crate_detail,
                list_crate.old_registry(),
                "REGISTRY OLD CRATE",
            );
            list_crate_type(crate_detail, list_crate.old_git(), "GIT OLD CRATE");
        }
        if list_subcommand.is_present("orphan") {
            list_crate_type(
                crate_detail,
                list_crate.orphan_registry(),
                "REGISTRY ORPHAN CRATE",
            );
            list_crate_type(crate_detail, list_crate.orphan_git(), "GIT ORPHAN CRATE");
        }
        if list_subcommand.is_present("used") {
            list_crate_type(
                crate_detail,
                list_crate.used_registry(),
                "REGISTRY USED CRATE",
            );
            list_crate_type(crate_detail, list_crate.used_git(), "GIT USED CRATE");
        }
        if list_subcommand.is_present("all") {
            list_crate_type(
                crate_detail,
                list_crate.installed_registry(),
                "REGISTRY INSTALLED CRATE",
            );
            list_crate_type(
                crate_detail,
                list_crate.installed_git(),
                "GIT INSTALLED CRATE",
            );
        }
    }
}

// list certain crate type to terminal
fn list_crate_type(crate_detail: &CrateDetail, crate_type: &[String], title: &str) {
    show_title(title);

    let mut total_size = 0.0;
    for crates in crate_type {
        let size = crate_detail.find(crates, title);
        total_size += size;
        println!("|{:^40}|{:^10.3}|", crates, size);
    }

    show_total_count(crate_type, total_size);
}

// show title
fn show_title(title: &str) {
    print_dash();
    println!("|{:^40}|{:^10}|", title.bold(), "SIZE(MB)");
    print_dash();
}

// show total count using data and size
fn show_total_count(data: &[String], size: f64) {
    if data.is_empty() {
        println!("|{:^40}|{:^10}|", "NONE".red(), "0.000".red());
    }
    print_dash();
    println!(
        "|{:^40}|{:^10}|",
        format!("Total no of crates:- {}", data.len()).bright_blue(),
        format!("{:.3}", size).bright_blue()
    );
    print_dash();
}

// print dash
fn print_dash() {
    println!(
        "{}",
        "-----------------------------------------------------"
            .green()
            .bold()
    );
}

// Clean old crates
fn old_clean(
    list_crate: &CrateList,
    (old_app, old_registry, old_git): (bool, bool, bool),
    registry_crates_location: &mut RegistryDir,
    git_crates_location: &GitDir,
    crate_detail: &CrateDetail,
) {
    if old_app || old_registry || old_git {
        let mut size_cleaned = 0.0;
        if old_app || old_registry {
            size_cleaned += registry_crates_location
                .remove_crate_list(&crate_detail, list_crate.old_registry());
        }
        if old_app || old_git {
            size_cleaned +=
                git_crates_location.remove_crate_list(&crate_detail, list_crate.old_git());
        }
        println!(
            "{}",
            format!("Total size of old crates removed :- {:.3} MB", size_cleaned).bright_blue()
        );
    }
}

// Clean orphan crates
fn orphan_clean(
    list_crate: &CrateList,
    (orphan_app, orphan_git, orphan_registry): (bool, bool, bool),
    registry_crates_location: &mut RegistryDir,
    git_crates_location: &GitDir,
    crate_detail: &CrateDetail,
) {
    if orphan_app || orphan_git || orphan_registry {
        let mut size_cleaned = 0.0;
        if orphan_app || orphan_registry {
            size_cleaned += registry_crates_location
                .remove_crate_list(&crate_detail, list_crate.orphan_registry());
        }
        if orphan_app || orphan_git {
            size_cleaned +=
                git_crates_location.remove_crate_list(&crate_detail, list_crate.orphan_git());
        }
        println!(
            "{}",
            format!(
                "Total size of orphan crates removed :- {:.3} MB",
                size_cleaned
            )
            .bright_blue()
        );
    }
}

// query size of directory
fn query_size(
    dir_path: &DirPath,
    (query_size_app, query_size_git, query_size_registry): (bool, bool, bool),
    crate_list: &CrateList,
    crate_detail: &CrateDetail,
) {
    let mut final_size = 0_u64;
    if query_size_app || query_size_git || query_size_registry {
        if query_size_app {
            let bin_dir_size = get_size(dir_path.bin_dir()).unwrap_or(0_u64);
            final_size += bin_dir_size;
            println!(
                "{:50} {:>10}",
                format!(
                    "Total size of {} .cargo/bin binary:",
                    crate_list.installed_bin().len()
                ),
                convert(bin_dir_size as f64)
            );
            print_dash();
        }
        if query_size_app || query_size_git {
            let git_dir_size = get_size(dir_path.git_dir()).unwrap_or(0_u64);
            final_size += git_dir_size;
            println!(
                "{:50} {:>10}",
                format!(
                    "Total size of {} .cargo/git crates:",
                    crate_list.installed_git().len()
                ),
                convert(git_dir_size as f64)
            );
            println!(
                "{:50} {:>10}",
                format!(
                    "   \u{251c} Size of {} .cargo/git/checkout folder",
                    crate_detail.git_crates_archive().len()
                ),
                convert(get_size(dir_path.checkout_dir()).unwrap_or(0_u64) as f64)
            );
            println!(
                "{:50} {:>10}",
                format!(
                    "   \u{2514} Size of {} .cargo/git/db folder",
                    crate_detail.git_crates_source().len()
                ),
                convert(get_size(dir_path.db_dir()).unwrap_or(0_u64) as f64)
            );
            print_dash();
        }
        if query_size_app || query_size_registry {
            let registry_dir_size =
                get_size(dir_path.registry_dir()).expect("failed to get size of registry dir");
            final_size += registry_dir_size;
            println!(
                "{:50} {:>10}",
                format!(
                    "Total size of {} .cargo/registry crates:",
                    crate_list.installed_registry().len()
                ),
                convert(registry_dir_size as f64)
            );
            println!(
                "{:50} {:>10}",
                format!(
                    "   \u{251c} Size of {} .cargo/registry/cache folder",
                    crate_detail.registry_crates_archive().len()
                ),
                convert(get_size(dir_path.cache_dir()).unwrap_or(0_u64) as f64)
            );
            println!(
                "{:50} {:>10}",
                "   \u{251c} Size of .cargo/registry/index folder",
                convert(get_size(dir_path.index_dir()).unwrap_or(0_u64) as f64)
            );
            println!(
                "{:50} {:>10}",
                format!(
                    "   \u{2514} Size of {} .cargo/git/src folder",
                    crate_detail.registry_crates_source().len()
                ),
                convert(get_size(dir_path.src_dir()).unwrap_or(0_u64) as f64)
            );
            print_dash();
        }
        println!(
            "{:50} {:>10}",
            format!(
                "Total size occupied by {}",
                std::env::var("CARGO_HOME").expect("No environmental variable CARGO_HOME present")
            ),
            convert(final_size as f64)
        );
    }
}

// Perform query about config file data
fn config_subcommand(app: &ArgMatches, config_file: &ConfigFile) {
    if app.is_present("config") {
        let matches = app.subcommand_matches("config").unwrap();
        let read_include = config_file.include();
        let read_exclude = config_file.exclude();
        let read_directory = config_file.directory();
        if matches.is_present("directory") {
            for name in read_directory {
                println!("{}", name);
            }
        }
        if matches.is_present("include") {
            for name in read_include {
                println!("{}", name);
            }
        }
        if matches.is_present("exclude") {
            for name in read_exclude {
                println!("{}", name);
            }
        }
    }
}

// force remove all crates
fn force_remove(
    dir_path: &DirPath,
    (force_remove_app, force_remove_git, force_remove_registry): (bool, bool, bool),
    (dry_run_app, dry_run_git, dry_run_registry): (bool, bool, bool),
) {
    if force_remove_app || force_remove_git || force_remove_registry {
        if force_remove_app || force_remove_registry {
            let dry_run = dry_run_app || dry_run_registry;
            delete_folder(&dir_path.cache_dir(), dry_run);
            delete_folder(&dir_path.src_dir(), dry_run);
            // Delete out .cache folder also
            let index_path = dir_path.index_dir();
            for entry in fs::read_dir(index_path)
                .expect("Failed to read index directory during force remove")
            {
                let entry = entry.unwrap().path();
                let registry_dir = entry.as_path();
                for folder in fs::read_dir(registry_dir)
                    .expect("Failed to read registry directory in force remove")
                {
                    let folder = folder.unwrap().path();
                    let folder_name = folder.file_name().unwrap();
                    if folder_name == ".cache" {
                        delete_folder(&folder, dry_run);
                    }
                }
            }
        }
        if force_remove_app || force_remove_git {
            let dry_run = dry_run_app || dry_run_git;
            delete_folder(&dir_path.checkout_dir(), dry_run);
            delete_folder(&dir_path.db_dir(), dry_run);
        }
        println!("{}", "Successfully removed all crates".red());
    }
}

// remove all crates by following config file information
fn remove_all(
    list_crate: &CrateList,
    config_file: &ConfigFile,
    registry_crates_location: &mut RegistryDir,
    git_crates_location: &GitDir,
    (all_app, all_git, all_registry): (bool, bool, bool),
    crate_detail: &CrateDetail,
) {
    if all_app || all_git || all_registry {
        let mut total_size_cleaned = 0.0;
        if all_app || all_registry {
            for crate_name in list_crate.installed_registry() {
                total_size_cleaned +=
                    registry_crates_location.remove_all(&config_file, crate_name, crate_detail);
            }
        }
        if all_app || all_git {
            for crate_name in list_crate.installed_git() {
                total_size_cleaned +=
                    git_crates_location.remove_all(&config_file, crate_name, crate_detail);
            }
        }
        println!(
            "{}",
            format!(
                "Total size of crates removed :- {:.3} MB",
                total_size_cleaned
            )
            .bright_blue()
        );
    }
}

// Remove certain crates
fn remove_crate(
    list_crate: &CrateList,
    (app, git_subcommand, registry_subcommand): (&ArgMatches, &ArgMatches, &ArgMatches),
    registry_crates_location: &mut RegistryDir,
    git_crates_location: &GitDir,
    crate_detail: &CrateDetail,
) {
    let remove_crate_app = app.is_present("remove-crate");
    let remove_crate_git = git_subcommand.is_present("remove-crate");
    let remove_crate_registry = registry_subcommand.is_present("remove-crate");
    if remove_crate_app || remove_crate_git || remove_crate_registry {
        let mut size_cleaned = 0.0;
        let value = app.value_of("remove-crate").unwrap_or_else(|| {
            git_subcommand
                .value_of("remove-crate")
                .unwrap_or_else(|| registry_subcommand.value_of("remove-crate").unwrap())
        });
        if list_crate.installed_registry().contains(&value.to_string())
            && (remove_crate_app || remove_crate_registry)
        {
            registry_crates_location.remove_crate(value);
            size_cleaned += crate_detail.find_size_registry_all(value);
        }

        if list_crate.installed_git().contains(&value.to_string())
            && (remove_crate_app || remove_crate_git)
        {
            git_crates_location.remove_crate(value);
            size_cleaned += crate_detail.find_size_git_all(value);
        }
        println!(
            "{}",
            format!("Total size removed :- {:.3} MB", size_cleaned).bright_blue()
        );
    }
}

// show out top n crates
fn top_crates(
    app: &ArgMatches,
    git_subcommand: &ArgMatches,
    registry_subcommand: &ArgMatches,
    crate_detail: &CrateDetail,
) {
    let top_app = app.is_present("top crates");
    let top_git = git_subcommand.is_present("top crates");
    let top_registry = registry_subcommand.is_present("top crates");
    if top_app || top_git || top_registry {
        let value = app.value_of("top crates").unwrap_or_else(|| {
            git_subcommand
                .value_of("top crates")
                .unwrap_or_else(|| registry_subcommand.value_of("top crates").unwrap())
        });

        let number = value
            .parse::<usize>()
            .expect("Cannot convert top n crates value in usize");
        if top_app {
            show_top_number_crates(crate_detail, "bin", number);
        }
        if top_app || top_git {
            show_top_number_crates(crate_detail, "git_archive", number);
            show_top_number_crates(crate_detail, "git_source", number);
        }
        if top_app || top_registry {
            show_top_number_crates(crate_detail, "registry_archive", number);
            show_top_number_crates(crate_detail, "registry_source", number);
        }
    }
}

// top_crates() help to list out top n crates
fn show_top_number_crates(crate_detail: &CrateDetail, crate_type: &str, number: usize) {
    let blank_hashmap = HashMap::new();
    let size_detail = match crate_type {
        "bin" => crate_detail.bin(),
        "git_archive" => crate_detail.git_crates_archive(),
        "git_source" => crate_detail.git_crates_source(),
        "registry_archive" => crate_detail.registry_crates_archive(),
        "registry_source" => crate_detail.registry_crates_source(),
        _ => &blank_hashmap,
    };
    let mut vector = size_detail.iter().collect::<Vec<_>>();
    vector.sort_by(|a, b| (b.1).cmp(a.1));
    let title = format!("Top {} {}", number, crate_type);
    show_title(title.as_str());
    if vector.is_empty() {
        println!("|{:^40}|{:^10}|", "NONE".red(), "0.000".red());
    } else if vector.len() < number {
        for i in 0..vector.len() {
            print_index_value_crate(&vector, i);
        }
    } else {
        for i in 0..number {
            print_index_value_crate(&vector, i);
        }
    }
    print_dash();
}

// print crate name
fn print_index_value_crate(vector: &[(&String, &u64)], i: usize) {
    let crate_name = vector[i].0;
    let size = vector[i].1;
    let size = (*size as f64) / 1024_f64.powf(2.0);
    println!("|{:^40}|{:^10.3}|", crate_name, size);
}

// Update cargo lock
fn update_cargo_toml(app: &ArgMatches, cargo_toml_location: &[PathBuf]) {
    if app.is_present("update") {
        for location in cargo_toml_location {
            let mut cargo_lock = location.clone();
            cargo_lock.push("Cargo.lock");
            // at first try generating lock file
            if !cargo_lock.exists() {
                if let Err(e) = Command::new("cargo")
                    .arg("generate-lockfile")
                    .current_dir(location)
                    .output()
                {
                    panic!(format!("Failed to generate Cargo.lock {}", e));
                }
            }
            // helps so we may not need to generate lock file again for workspace project
            if cargo_lock.exists() {
                let message = format!("Updating {}", cargo_lock.to_str().unwrap().bright_blue());
                println!("{}", message);
                if let Err(e) = Command::new("cargo")
                    .arg("update")
                    .current_dir(location)
                    .output()
                {
                    panic!(format!("Failed to update Cargo.lock {}", e));
                }
            }
        }
        println!("{}", "Successfully update all Cargo.lock".bright_blue());
    }
}

// Wipe certain directory
fn wipe_directory(app: &ArgMatches, dir_path: &DirPath) {
    if app.is_present("wipe") {
        let value = app.value_of("wipe").unwrap();
        let dry_run = app.is_present("dry run");
        match value {
            "git" => delete_folder(&dir_path.git_dir(), dry_run),
            "checkouts" => delete_folder(&dir_path.checkout_dir(), dry_run),
            "db" => delete_folder(&dir_path.db_dir(), dry_run),
            "registry" => delete_folder(&dir_path.registry_dir(), dry_run),
            "cache" => delete_folder(&dir_path.cache_dir(), dry_run),
            "index" => delete_folder(&dir_path.index_dir(), dry_run),
            "src" => delete_folder(&dir_path.src_dir(), dry_run),
            _ => (),
        }
    }
}

// delete folder with folder path provided
fn delete_folder(path: &PathBuf, dry_run: bool) {
    if path.exists() {
        if dry_run {
            println!("{} {} {:?}", "Dry run:".yellow(), "removed".red(), path);
        } else {
            fs::remove_dir_all(path).expect("failed to remove all directory content");
            println!("{} {:?}", "Removed".red(), path);
        }
    }
}
