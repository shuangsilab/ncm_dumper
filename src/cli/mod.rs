use std::num::NonZeroU32;
use std::path::PathBuf;

use super::Config;
use anyhow::Context;
use encoding_rs::{GBK, UTF_8};
use walkdir::WalkDir;

pub mod en_us;
pub mod zh_cn;

#[allow(non_camel_case_types)]
enum Lang {
    zh_CN,
    en_US,
}

pub fn run() -> Config {
    let lang = match std::env::var("LANG") {
        Err(_) => Lang::en_US,
        Ok(lang) => match lang.split_once('.') {
            Some(("zh_CN", _)) => Lang::zh_CN,
            _ => Lang::en_US,
        },
    };

    return match lang {
        Lang::zh_CN => zh_cn::run(),
        Lang::en_US => en_us::run(),
    };
}

#[derive(Debug)]
pub struct ErrMsg {
    pub header: &'static str,
    pub invalid_utf8: &'static str,
    pub get_path_meta: &'static str,
    pub walkdir: &'static str,
    pub no_output: &'static str,

    pub reading_file: &'static str,
    pub saving_ncm: &'static str,
    pub saving_img: &'static str,
    pub saving_meta: &'static str,
    pub not_ncm: &'static str,
    pub parsing_ncm: &'static str,

    pub ok_msg: &'static str,
}

macro_rules! UTF_8DEC {
    ($x: expr) => {
        UTF_8.decode_without_bom_handling_and_without_replacement($x)
    };
}

macro_rules! GBKDEC {
    ($x: expr) => {
        GBK.decode_without_bom_handling_and_without_replacement($x)
    };
}

trait CLIConfig {
    const ERR_MSG: &'static ErrMsg;

    fn inputs(&self) -> Option<&Vec<String>>;
    fn filelists(&self) -> Option<&Vec<String>>;
    fn output_dir(&self) -> Option<&String>;
    fn dir_recursive(&self) -> bool;
    fn no_music(&self) -> bool;
    fn cover_img(&self) -> bool;
    fn metadata(&self) -> bool;
    fn threads(&self) -> u32;
    fn skip_error(&self) -> bool;

    fn error(&self, err_msg: std::fmt::Arguments) {
        eprintln!("{} {}", Self::ERR_MSG.header, err_msg);
        if self.skip_error() == false {
            std::process::exit(1);
        }
    }

    fn config(&self) -> Config {
        let err_msg = &Self::ERR_MSG;

        if self.no_music() == true
            && self.metadata() == false
            && self.cover_img() == false
        {
            self.error(format_args!("{}", err_msg.no_output));
        }

        let mut ncm_dirs = Vec::new();
        let mut ncm_files = Vec::new();

        let empty_vec = Vec::new();
        let filelists = self.filelists().unwrap_or(&empty_vec);
        for file in filelists {
            let file_txt = match std::fs::read(file)
                .context(format!("{} [{}]", err_msg.reading_file, file))
            {
                Ok(file_txt) => file_txt,
                Err(err) => {
                    self.error(format_args!("{err:?}"));
                    continue;
                }
            };

            let pathlist = if let Some(pathlist) = UTF_8DEC!(&file_txt) {
                pathlist
            } else if let Some(pathlist) = GBKDEC!(&file_txt) {
                pathlist
            } else {
                self.error(format_args!("{} [{}]", err_msg.invalid_utf8, file));
                continue;
            };

            for path in pathlist.lines().map(|x| PathBuf::from(x)) {
                match path.metadata().context(format!(
                    "{} [{}] [{}]",
                    err_msg.get_path_meta,
                    path.display(),
                    file,
                )) {
                    Ok(metadata) => {
                        if metadata.is_file() {
                            ncm_files.push(path)
                        } else
                        /* metadata.is_dir() == true */
                        {
                            // According to the standard library,
                            // the two conditions are mutually exclusive
                            ncm_dirs.push(path)
                        }
                    }
                    Err(err) => {
                        self.error(format_args!("{err:?}"));
                        continue;
                    }
                }
            }
        }

        let pathlist = self.inputs().unwrap_or(&empty_vec);
        for path in pathlist {
            let path = PathBuf::from(path);
            match path.metadata().context(format!(
                "{} [{}]",
                err_msg.get_path_meta,
                path.display()
            )) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        ncm_files.push(path)
                    } else
                    /* metadata.is_dir() == true */
                    {
                        // According to the standard library,
                        // the two conditions are mutually exclusive
                        ncm_dirs.push(path)
                    }
                }
                Err(err) => {
                    self.error(format_args!("{err:?}"));
                    continue;
                }
            }
        }

        for dir in ncm_dirs {
            let mut wdir = WalkDir::new(&dir);
            if self.dir_recursive() == false {
                wdir = wdir.max_depth(1);
            }

            let files: Vec<_> = match wdir
                .into_iter()
                .try_collect()
                .context(format!("{} [{}]", err_msg.walkdir, dir.display()))
            {
                Ok(files) => files,
                Err(err) => {
                    self.error(format_args!("{err:?}"));
                    continue;
                }
            };

            ncm_files.extend(
                files
                    .into_iter()
                    .map(|entry| entry.into_path())
                    .filter(|path| path.extension() == Some("ncm".as_ref())),
            );
        }

        return Config {
            err_msg,
            ncm_files,
            output_dir: self.output_dir().as_ref().map(|path| PathBuf::from(path)),
            threads: NonZeroU32::new(self.threads()),
            skip_error: self.skip_error(),
            with_music: !self.no_music(),
            with_image: self.cover_img(),
            with_metadata: self.metadata(),
        };
    }
}
