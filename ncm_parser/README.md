# ncm_parser

A single file **ncm** parser. Here, the **ncm** is an encrypted
music file format which is widely used on **NeteaseCloudMuic**.

For more details see
[https://www.cnblogs.com/cyx-b/p/13443003.html](https://www.cnblogs.com/cyx-b/p/13443003.html)

一个单文件实现的 .ncm 解析器。

# Examples
```rust
use ncm_parser::NCMMetadata;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

fn main() {
    // Open .ncm file
    let mut ncm_file = Vec::new();
    let ncm_file_name = Path::new("xxx.ncm");
    File::open(ncm_file_name)
        .unwrap()
        .read_to_end(&mut ncm_file)
        .unwrap();

    // Parse ncm file with `from_iter`
    let mut ncm_file_from_iter =
        ncm_parser::from_iter(ncm_file.into_iter()).unwrap();

    // Directly parse ncm file with `from_reader`
    let mut ncm_file_from_reader =
        ncm_parser::from_reader(File::open(ncm_file_name).unwrap()).unwrap();

    // Two methods are identital.
    assert_eq!(
        ncm_file_from_iter.get_image(),
        ncm_file_from_reader.get_image()
    );
    assert_eq!(
        ncm_file_from_iter.get_metadata().unwrap(),
        ncm_file_from_reader.get_metadata().unwrap()
    );
    assert_eq!(
        ncm_file_from_iter.get_music().unwrap(),
        ncm_file_from_reader.get_music().unwrap()
    );

    let image = ncm_file_from_iter.get_image_unchecked();
    let metadata = ncm_file_from_iter.get_metadata_unchecked();
    let music = ncm_file_from_iter.get_music_unchecked();

    // Parse metadata
    let ncm_meta = NCMMetadata::new(metadata).unwrap();

    // Save music
    let music_name = ncm_file_name.with_extension(&ncm_meta.format);
    File::create(music_name).unwrap().write_all(&music).unwrap();

    // Read the cover image format
    let image_ext = ncm_meta.album_pic_url.rsplit_once('.').unwrap().1;

    // Save cover image
    let image_name = ncm_file_name.with_extension(image_ext);
    File::create(image_name).unwrap().write_all(image).unwrap();

    // Save metadata
    let meta_name = ncm_file_name.with_extension("json");
    File::create(meta_name).unwrap().write_all(metadata).unwrap();
}
```