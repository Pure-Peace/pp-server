use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use md5::{Digest, Md5};
use std::path::Path;
use std::{fs, io};
use std::{
    fs::File,
    io::{BufReader, Read},
    time::Instant,
};

use crate::settings::model::Settings;

#[inline(always)]
pub fn safe_string(mut s: String) -> String {
    for i in r#":\*></?"|.,()[]{}!@#$%^&-_=+~`"#.chars() {
        s = s.replace(i, "");
    }
    s
}

#[inline(always)]
pub fn check_is_osu_file(entry: &Result<fs::DirEntry, io::Error>) -> u8 {
    if entry.is_err() {
        return 3;
    };
    let entry = entry.as_ref().unwrap();
    if entry.path().is_dir() {
        return 2;
    };
    let file_name = match entry.file_name().into_string() {
        Ok(n) => n,
        Err(_) => {
            return 3;
        }
    };
    if !file_name.ends_with(".osu") {
        return 3;
    };
    1
}

#[inline(always)]
pub fn calc_file_md5<P: AsRef<Path>>(path: P) -> Result<String, io::Error> {
    let file = File::open(path)?;
    let mut hasher = Md5::new();
    let mut buffer = [0; 8192];
    let mut reader = BufReader::new(file);
    while let Ok(size) = reader.read(&mut buffer) {
        if size == 0 {
            break;
        };
        hasher.update(&buffer[..size]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[inline(always)]
pub fn recalculate_osu_file_md5(osu_files_dir: String) {
    println!(
        "{}",
        format!("\n> Listing .osu dir '{}' now...", osu_files_dir).bright_yellow()
    );
    let mut renamed = 0;
    let mut done = 0;
    let mut dirs = 0;
    let mut error = 0;
    let entries: Vec<Option<fs::DirEntry>> = fs::read_dir(osu_files_dir.clone())
        .unwrap()
        .map(|r| match check_is_osu_file(&r) {
            1 => Some(r.unwrap()),
            2 => {
                dirs += 1;
                None
            }
            _ => {
                error += 1;
                None
            }
        })
        .filter(|r| r.is_some())
        .collect();
    let total = entries.len();
    println!(
        "\n{}",
        format!("> Done, .osu file count: {}", total).bright_yellow()
    );
    println!("\n  Recalculating MD5 file names...");
    let bar = ProgressBar::new(total as u64);
    bar.set_style(ProgressStyle::default_bar().template("{spinner:.green} [{elapsed_precise}] [{bar:40.green/white} ]{pos:>7}/{len:7} ({eta})").progress_chars("#>-"));
    let start = Instant::now();
    for entry in entries {
        if let Some(entry) = entry {
            let md5 = match calc_file_md5(entry.path()) {
                Ok(md5) => md5,
                Err(_) => {
                    error += 1;
                    continue;
                }
            };
            if fs::rename(entry.path(), format!("{}/{}.osu", osu_files_dir, md5)).is_err() {
                error += 1;
            } else {
                renamed += 1;
            }
            done += 1;
            bar.inc(1);
        }
    }
    bar.finish();
    println!(
        "{}\n",
        format!(
            "> Done, \n> Success / Done / Total: {} / {} / {}; \n> Errors / Dirs: {} / {}; \n> time spent: {:?}",
            renamed,
            done,
            total,
            error,
            dirs,
            start.elapsed()
        )
        .bright_yellow()
    )
}

#[inline(always)]
pub fn checking_osu_dir(data: &Settings) {
    if data.osu_files_dir == "" {
        println!(
            "{}",
            "> [Error] Please add .osu files dir in pp-server-config!!!\n"
                .bold()
                .red()
        );
    } else if data.recalculate_osu_file_md5 {
        recalculate_osu_file_md5(data.osu_files_dir.clone());
    };
}
