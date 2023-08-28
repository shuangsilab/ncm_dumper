use std::fs::File;
use std::path::PathBuf;

use anyhow::{Context, Result};
use ncm_parser::{self, ParseError};

use crate::cli::ErrMsg;

pub fn dump(
    err_msg: &ErrMsg,
    file: &'static PathBuf,
    out_dir: Option<&PathBuf>,
    with_music: bool,
    with_image: bool,
    with_metadata: bool,
) -> Result<(&'static str, &'static PathBuf)> {
    let in_file = File::open(&file).context(format!(
        "{} [{}]",
        err_msg.reading_file,
        file.display()
    ))?;

    let mut ncm = match ncm_parser::from_reader(in_file) {
        Ok(ncm) => ncm,
        err @ Err(ParseError::InvalidHeader) => {
            err.context(format!("{} [{}]", err_msg.not_ncm, file.display()))?
        }
        err @ _ => {
            err.context(format!("{} [{}]", err_msg.parsing_ncm, file.display()))?
        }
    };

    let out_file_exts_with_ncm = match out_dir {
        Some(out_dir) => out_dir.join(file.file_name().unwrap()),
        None => file.clone(),
    };

    let metadata = ncm.get_parsed_metadata().context(format!(
        "{} [{}]",
        err_msg.parsing_ncm,
        file.display()
    ))?;

    if with_music {
        let music = ncm.get_music().context(format!(
            "{} [{}]",
            err_msg.parsing_ncm,
            file.display()
        ))?;

        let out_file = out_file_exts_with_ncm.with_extension(metadata.format);
        std::fs::write(&out_file, music).context(format!(
            "{} [{}]",
            err_msg.saving_ncm,
            out_file.display()
        ))?;
    }

    if with_image {
        let image = ncm.get_image().into_ok();
        let out_file = out_file_exts_with_ncm.with_extension(
            metadata
                .album_pic_url
                .rsplit_once('.')
                .context(format!("{} [{}]", err_msg.saving_img, file.display()))?
                .1,
        );
        std::fs::write(&out_file, image).context(format!(
            "{} [{}]",
            err_msg.saving_img,
            out_file.display()
        ))?;
    }

    if with_metadata {
        let metadata = ncm.get_metadata_unchecked();
        let out_file = out_file_exts_with_ncm.with_extension("json");
        std::fs::write(&out_file, metadata).context(format!(
            "{} [{}]",
            err_msg.saving_meta,
            out_file.display()
        ))?;
    }

    Ok((err_msg.ok_msg, file))
}
