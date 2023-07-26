use clap::{command, Args, Parser};
use ncm_parser::NCMMetadata;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use thiserror::Error;
use yansi::Color::Red;
use yansi::Paint;

const HELP_TEMPLATE: &str = "
{name} {version} <https://github.com/shuangsilab/ncm_dumper>

{about}.

{usage-heading} {usage}

{all-args}
";

#[derive(Parser, Debug)]
#[command(
    version,
    max_term_width = 80,
    disable_version_flag = true,
    arg_required_else_help = true,
    next_line_help = true,
    help_template = HELP_TEMPLATE,
    about,
)]

struct CommandLine {
    #[arg(
        short = 'i', long,
        value_name = "FILE|DIR",
        num_args = 1..,
        help_heading = "Input/Output",
        required_unless_present = "filelists",
        help = "\
            Specify *.ncm files or directories containing *.ncm files.\n\
            Example: -i \"1.ncm\" \"2.ncm\" \"C:\\dir1\" \"D:\\dir2\" ...\n\
        "
    )]
    inputs: Option<Vec<String>>,

    #[arg(
        short = 'f', long,
        value_name = "*.txt",
        num_args = 1..,
        help_heading = "Input/Output",
        help = "\
            Give a filelist containing PATH of <FILE> and <DIR> per line.\n\
            Example: -f filelist1.txt ...
        "
    )]
    filelists: Option<Vec<String>>,

    #[arg(
        short = 'o',
        long,
        value_name = "DIR",
        help_heading = "Input/Output",
        help = "\
            Specify the output directories. By default, \
            each output file is stored in the directory \
            where the corresponding input file is located.\n\
            Example: -d .\\out
        "
    )]
    output_dir: Option<String>,

    #[arg(
        short = 'r',
        long,
        help_heading = "Input/Output",
        help = "\
            Search *.ncm files in <DIR> recursively.
        "
    )]
    dir_recursive: bool,

    #[arg(
        short = 'n',
        long,
        help_heading = "OutputFlags",
        help = "\
            Don't output music file. By default, the output's \
            music file name will be same as the input's except \
            the file extension which would be .mp3 or .flac
        "
    )]
    no_music: bool,
    #[arg(
        short = 'c',
        long,
        help_heading = "OutputFlags",
        help = "\
            Output cover image. By default, the output's \
            image file name will be same as the input's except \
            file extension which would be .jpg or .png
        "
    )]
    cover_img: bool,
    #[arg(
        short = 'm',
        long,
        help_heading = "OutputFlags",
        help = "\
            Output ncm metadata. By default, the output's \
            metadata file name will be same as the input's except \
            file extension which would be .json
        "
    )]
    metadata: bool,

    #[arg(
        short = 't',
        long,
        value_name = "0..255",
        help = "Set the number of parallel tasks to run. 0 for auto."
    )]
    // The actual max number of theads is platform specific.
    // Typically, it varies from a few hundred to tens of thousands.
    threads: Option<usize>,
}

struct Config {
    files_path: Vec<PathBuf>,
    threads: usize,
    // 001: with_music
    // 010: with_image
    // 100: with_metadata
    with_outputs_flags: u8,
}

fn parse_cli() -> Config {
    // If error, directly exit
    let cli = CommandLine::parse();

    todo!()
}

#[derive(Debug)]
struct ErrContext {
    file_name: PathBuf,
    error: Box<dyn std::error::Error>,
}

fn work_unit(
    file_name: PathBuf,
    with_music: bool,
    with_image: bool,
    with_metadata: bool,
) -> Result<(), ErrContext> {
    // The single thread ncm parser.

    let ncm_file = match File::open(&file_name) {
        Ok(ncm_file) => ncm_file,
        Err(err) => {
            return Err(ErrContext {
                file_name,
                error: err.into(),
            })
        }
    };

    let mut ncm = match ncm_parser::from_reader(ncm_file) {
        Ok(ncm) => ncm,
        Err(err) => {
            return Err(ErrContext {
                file_name,
                error: err.into(),
            })
        }
    };

    let metadata = match ncm.get_metadata() {
        Ok(metadata) => metadata,
        Err(err) => {
            return Err(ErrContext {
                file_name,
                error: err.into(),
            })
        }
    };

    let Some(json_meta) = NCMMetadata::new(&metadata) else {
        return Err(ErrContext {
            file_name,
            error: "Parse ncm metadata into struct failed.".into(),
        });
    };

    if with_metadata {
        let meta_file = match File::open(file_name.with_extension("json")) {
            Ok(meta_file) => meta_file,
            Err(err) => return Err(ErrContext {
                file_name,
                error: "Parse ncm metadata into struct failed.".into(),
            });
        }
    }

    todo!()
}

fn distribute_works(config: Config) {
    // If error, directly exit
}

fn console_outputs(recv: Receiver<u32>) {}

fn main() {
    // let config = parse_cli();
    // distribute_works(config);

    macro_rules! error {
        ($x:expr) => {
            eprintln!("{}: {}", yansi::Paint::red("error").bold(), $x);
        };
    }

    error!("first try");
}
