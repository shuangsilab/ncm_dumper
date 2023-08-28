#![feature(iterator_try_collect)]
#![feature(unwrap_infallible)]
use rusty_pool;
use std::num::NonZeroU32;
use std::path::PathBuf;

mod cli;
mod dump;

#[derive(Debug)]
pub struct Config {
    pub err_msg: &'static cli::ErrMsg,
    pub ncm_files: Vec<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub threads: Option<NonZeroU32>,
    pub skip_error: bool,
    pub with_music: bool,
    pub with_image: bool,
    pub with_metadata: bool,
}

fn main() {
    let cfg = Box::leak(Box::new(cli::run()));

    let thread_pool = match cfg.threads {
        Some(threads) => rusty_pool::Builder::default()
            .max_size(threads.get() as usize)
            .build(),
        None => rusty_pool::Builder::default().build(),
    };

    let mut tasks = Vec::new();
    for file in cfg.ncm_files.iter() {
        let task = || {
            dump::dump(
                cfg.err_msg,
                file,
                cfg.output_dir.as_ref(),
                cfg.with_music,
                cfg.with_image,
                cfg.with_metadata,
            )
        };
        tasks.push(thread_pool.evaluate(task));
    }

    let len = tasks.len();
    for (i, task) in tasks.into_iter().enumerate() {
        match task.await_complete(){
            Ok((ok_msg, file_name)) => {
                println!("[{}/{}] {} [{}]", i + 1, len, ok_msg, file_name.display());
            }
            Err(err) => {
                eprintln!("{} {:?}", cfg.err_msg.header, err);
                if cfg.skip_error == false {
                    thread_pool.shutdown();
                    break;
                }
            }
        }
    }
}