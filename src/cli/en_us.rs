use super::{CLIConfig, Config, ErrMsg};
use clap::Parser;

#[rustfmt::skip]
const HELP_TEMPLATE: &str = "
{name} {version} <https://github.com/shuangsilab/ncm_dumper>

{about}.

{usage-heading} {usage}

{all-args}
";

#[rustfmt::skip]
#[derive(Parser, Debug)]
#[command(
    about,
    version,
    max_term_width = 80,
    disable_version_flag = true,
    arg_required_else_help = true,
    next_line_help = true,
    help_template = HELP_TEMPLATE,
)]
pub struct CLI {
    #[arg(
        short, long,
        value_name = "FILE|DIR",
        num_args = 1..,
        help_heading = "Input/Output",
        required_unless_present = "filelists",
        help = "\
            Specify paths of *.ncm files or directories containing *.ncm files.\n\
            Example: -i \"1.ncm\" \"2.ncm\" \"C:\\dir1\" \"D:\\dir2\" ...\n\
        "
    )]
    inputs: Option<Vec<String>>,

    #[arg(
        short, long,
        value_name = "*.txt",
        num_args = 1..,
        help_heading = "Input/Output",
        help = "\
            Give a filelist containing PATH of <FILE> and <DIR> per line.\n\
            Example: -f filelist.txt ...
        "
    )]
    filelists: Option<Vec<String>>,

    #[arg(
        short = 'd',
        long,
        value_name = "DIR",
        help_heading = "Input/Output",
        help = "\
            Specify the output directory. By default, each output file is stored in \
            the directory where the corresponding input file is located.\n\
            Example: -d .\\out
        "
    )]
    output_dir: Option<String>,

    #[arg(
        short = 'r',
        long,
        help_heading = "Input/Output",
        help = "Search *.ncm files in <DIR> recursively."
    )]
    dir_recursive: bool,

    #[arg(
        short,
        long,
        help_heading = "OutputFlag",
        help = "\
            Don't output music file. By default, the output's music file name will \
            be same as the input's except the file extension which would be \
            .mp3 or .flac
        "
    )]
    no_music: bool,
    #[arg(
        short,
        long,
        help_heading = "OutputFlag",
        help = "\
            Output cover image. By default, the output's image file name will be \
            same as the input's except file extension which would be .jpg or .png
        "
    )]
    cover_img: bool,
    #[arg(
        short,
        long,
        help_heading = "OutputFlag",
        help = "\
            Output ncm metadata. By default, the output's metadata file name will \
            be same as the input's except file extension which would be .json
        "
    )]
    metadata: bool,
    #[arg(
        short,
        long,
        value_name = "0..255",
        default_value = "0",
        help = "Set the number of parallel tasks to run. 0 for auto."
    )]
    // The actual max number of theads is platform specific.
    // Typically, it varies from a few hundred to tens of thousands.
    threads: u32,

    #[arg(short, long, help = "Don't exit when error occurs, just report it.")]
    skip_errors: bool,
}

impl CLIConfig for CLI {
    const ERR_MSG: ErrMsg = ErrMsg {
        header: "\x1b[1;91mError:\x1b[0m",
        filelist_read: "Failed in reading paths in filelist.",
        get_path_meta: "Failed in reading metadata of path.",
        walkdir: "Failed to read files in directory.",
        no_output: "No output when enabling '--no-music' only.",

        reading_file: "Failed in reading files.",
        saving_ncm: "Failed in saving ncm files.",
        saving_img: "Failed in saving cover image.",
        saving_meta: "Failed in saving metadata.",
        not_ncm: "This file is not a valid ncm file.",
        parsing_ncm: "Failed in parsing ncm files.",
    };

    fn inputs(&self) -> Option<&Vec<String>> {
        self.inputs.as_ref()
    }
    fn filelists(&self) -> Option<&Vec<String>> {
        self.filelists.as_ref()
    }
    fn output_dir(&self) -> Option<&String> {
        self.output_dir.as_ref()
    }
    fn dir_recursive(&self) -> bool {
        self.dir_recursive
    }
    fn no_music(&self) -> bool {
        self.no_music
    }
    fn cover_img(&self) -> bool {
        self.cover_img
    }
    fn metadata(&self) -> bool {
        self.metadata
    }
    fn threads(&self) -> u32 {
        self.threads
    }
    fn skip_error(&self) -> bool {
        self.skip_errors
    }
}

pub fn run() -> Config {
    let cli = CLI::parse();

    return cli.config();
}
