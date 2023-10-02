#![feature(path_file_prefix)]
#[test]
fn try_dump() {
    use std::ffi::OsStr;
    use std::fs::File;
    let test_dir = std::env::current_dir().unwrap();
    let test_dir = test_dir.join("tests");
    let json_dir = test_dir.join("json");
    let ncm_dir = test_dir.join("ncm");
    let txt_dir = test_dir.join("txt");
    let music_dir = test_dir.join("music");
    let img_dir = test_dir.join("img");

    let ncm_files = std::fs::read_dir(ncm_dir).unwrap();
    let ncm_files: Vec<_> = ncm_files
        .into_iter()
        .filter_map(|file| {
            let file_path = file.unwrap().path();
            if file_path.extension() == Some(OsStr::new("ncm")) {
                return Some(file_path);
            } else {
                return None;
            }
        })
        .collect();

    for file in ncm_files {
        let file_no_ext = file.file_prefix().unwrap();
        let file_name = file.file_name().unwrap().to_str().unwrap();
        println!("Open File: [{}]", file_name);
        let mut ncm = ncm_parser::from_reader(File::open(&file).unwrap()).unwrap();
        
        let meta = ncm.get_metadata().unwrap();

        let json_file = json_dir.join(file_no_ext).with_extension("json");
        std::fs::write(json_file, meta).unwrap();

        let parsed_meta = ncm.get_parsed_metadata().unwrap();

        let txt_file = txt_dir.join(file_no_ext).with_extension("txt");
        std::fs::write(txt_file, format!("{:#?}", parsed_meta)).unwrap();

        let music = ncm.get_music().unwrap();

        let music_file = music_dir.join(file_no_ext).with_extension(parsed_meta.format);
        std::fs::write(music_file, music).unwrap();
        
        let image = ncm.get_image().unwrap();

        let image_file = img_dir.join(file_no_ext).with_extension("jpg");
        std::fs::write(image_file, image).unwrap();
    }
}
