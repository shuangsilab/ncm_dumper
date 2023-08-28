use super::{CLIConfig, Config, ErrMsg};
use clap::Parser;

#[rustfmt::skip]
const HELP_TEMPLATE: &str = "
{name} {version} <https://github.com/shuangsilab/ncm_dumper>

多线程 ncm 文件解包工具

{usage-heading} {usage}

{all-args}
";

#[rustfmt::skip]
#[derive(Parser, Debug)]
#[command(
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
            输入 .ncm 文件的路径或包含 .ncm 文件的目录。\n\
            例如：-i \"1.ncm\" \"2.ncm\" \"C:\\dir1\" \"D:\\dir2\" ...\n\
        "
    )]
    inputs: Option<Vec<String>>,

    #[arg(
        short, long,
        value_name = "*.txt",
        num_args = 1..,
        help_heading = "Input/Output",
        help = "\
            输入一个文本文件，每一行表示文件或文件夹的路径\n\
            例如：-f filelist.txt ...
        "
    )]
    filelists: Option<Vec<String>>,

    #[arg(
        short = 'd',
        long,
        value_name = "DIR",
        help_heading = "Input/Output",
        help = "\
            指定输出目录。默认情况下输出文件和输入文件存放在同一个位置\n\
            例如：-d .\\out
        "
    )]
    output_dir: Option<String>,

    #[arg(
        short = 'r',
        long,
        help_heading = "Input/Output",
        help = "是否递归地搜索目录下的 .ncm 文件"
    )]
    dir_recursive: bool,

    #[arg(
        short,
        long,
        help_heading = "OutputFlag",
        help = "\
            不导出音频文件
        "
    )]
    no_music: bool,
    #[arg(
        short,
        long,
        help_heading = "OutputFlag",
        help = "\
            导出封面图片
        "
    )]
    cover_img: bool,
    #[arg(
        short,
        long,
        help_heading = "OutputFlag",
        help = "\
            导出文件元信息
        "
    )]
    metadata: bool,

    #[arg(
        short,
        long,
        value_name = "0..255",
        default_value = "0",
        hide_default_value = true,
        help = "设置最大并行解码的线程数量。[0]表示由软件自动设置"
    )]
    threads: u32,

    #[arg(short, long, help = "当发生错误时仅报错而不退出")]
    skip_errors: bool,
}

impl CLIConfig for CLI {
    const ERR_MSG: ErrMsg = ErrMsg {
        header: "\x1b[1;91m错误:\x1b[0m",
        filelist_read: "解析文件中的路径时发生错误：",
        get_path_meta: "读取路径信息时发生错误：",
        walkdir: "无法读取路径下的文件：",
        no_output: "仅启用 --no-music 选项的情况下程序将不会输出任何文件。",

        reading_file: "读取文件失败：",
        saving_ncm: "保存 ncm 文件时出错：",
        saving_img: "保存图片时出错：",
        saving_meta: "保存文件元信息时出错：",
        not_ncm: "不是 ncm 文件。",
        parsing_ncm: "解析 ncm 文件时出现错误：",
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
    // 命令行解析出现错误时仍然会冒出英文，要解决这个问题要么凭空多出一千行自己写一个Parser
    // 要么用 try_parse 多出三百行代码，匹配 err 的 context 后手动写错误信息
    // 但是这个 context 也是贼他妈抽象，鬼知道要怎么匹配

    let cli = CLI::parse();

    return cli.config();
}
