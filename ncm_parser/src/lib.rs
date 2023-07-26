//! A single file **ncm** parser. Here, the **ncm** is an encrypted
//! music file format which is widely used on **NeteaseCloudMuic**.
//!
//! For more details see
//! [https://www.cnblogs.com/cyx-b/p/13443003.html](https://www.cnblogs.com/cyx-b/p/13443003.html)
//!
//! 一个单文件实现的 .ncm 解析器。
//!
//! # Examples
//! ```
//! use ncm_parser::NCMMetadata;
//! use std::fs::File;
//! use std::io::{Read, Write};
//! use std::path::Path;
//!
//! fn main() {
//!     // Open .ncm file
//!     let mut ncm_file = Vec::new();
//!     let ncm_file_name = Path::new("xxx.ncm");
//!     File::open(ncm_file_name)
//!         .unwrap()
//!         .read_to_end(&mut ncm_file)
//!         .unwrap();
//!
//!     // Parse ncm file with `from_iter`
//!     let mut ncm_file_from_iter =
//!         ncm_parser::from_iter(ncm_file.into_iter()).unwrap();
//!
//!     // Directly parse ncm file with `from_reader`
//!     let mut ncm_file_from_reader =
//!         ncm_parser::from_reader(File::open(ncm_file_name).unwrap()).unwrap();
//!
//!     // Two methods are identital.
//!     assert_eq!(
//!         ncm_file_from_iter.get_image(),
//!         ncm_file_from_reader.get_image()
//!     );
//!     assert_eq!(
//!         ncm_file_from_iter.get_metadata().unwrap(),
//!         ncm_file_from_reader.get_metadata().unwrap()
//!     );
//!     assert_eq!(
//!         ncm_file_from_iter.get_music().unwrap(),
//!         ncm_file_from_reader.get_music().unwrap()
//!     );
//!
//!     let image = ncm_file_from_iter.get_image_unchecked();
//!     let metadata = ncm_file_from_iter.get_metadata_unchecked();
//!     let music = ncm_file_from_iter.get_music_unchecked();
//!
//!     // Parse metadata
//!     let ncm_meta = ncm_file_from_iter.get_parsed_matadata().unwrap();
//!
//!     // Save music
//!     let music_name = ncm_file_name.with_extension(&ncm_meta.format);
//!     File::create(music_name).unwrap().write_all(&music).unwrap();
//!
//!     // Read the cover image format
//!     let image_ext = ncm_meta.album_pic_url.rsplit_once('.').unwrap().1;
//!
//!     // Save cover image
//!     let image_name = ncm_file_name.with_extension(image_ext);
//!     File::create(image_name).unwrap().write_all(image).unwrap();
//!
//!     // Save metadata
//!     let meta_name = ncm_file_name.with_extension("json");
//!     File::create(meta_name).unwrap().write_all(metadata).unwrap();
//!
//!     println!("{:#?}", ncm_meta);
//! }
//! ```

#![feature(iter_next_chunk)]
#![feature(iter_advance_by)]
#![feature(iterator_try_collect)]
#![feature(doc_auto_cfg)]
#![warn(missing_docs)]
use std::io::Read;

use aes::Aes128Dec;
use base64::engine::general_purpose::STANDARD_NO_PAD as base64dec;
use base64::Engine;
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecrypt, KeyInit};
use thiserror::Error;

use ParseError::*;

/// An error type represents all the possible errors.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    #[error("The ncm file ends unexpectedly.")]
    /// The *.ncm file ends unexpectedly while inisializing [`NCMFile`]
    /// with [`from_iter()`] or [`from_reader()`]
    EndOfFile,
    #[error("The ncm file header does not match \"CTENFDAM\".")]
    /// The ncm file header does not match \"CTENFDAM\\x01\\x70",
    /// which indicates the input file may not be ncm format.
    InvalidHeader,
    #[error("Decrypt ncm RC4 key failed.")]
    /// Failed when decrypting the AES-128 encrypted RC4 key.
    /// We can't get the music data without correctly decrypted RC4 key.
    DecryptRC4KeyFailed,
    #[error("Decrypt ncm metadata failed.")]
    /// Failed when decrypting the AES-128 and BASE64 encrypted matadata.
    DecryptMetadataFailed,
    #[error("Parse ncm metadata into struct failed.")]
    /// Failed when parsing the JSON format metadata into struct.
    ParseMetadataFailed,
}

/// A wrapped function for reading data
/// +----------------------------------------------------------+
/// |                         segment                          |
/// +----------------------------+-----------------------------+
/// |        segment_len         |         segment_data        |
/// |  length_of_encrypted_data  |   encrypted_data_with_salt  |
/// +----------------------------+-----------------------------+
fn read_segment_iter<T>(iter: &mut T, salt: u8) -> Option<Vec<u8>>
where
    T: Iterator<Item = u8> + Clone,
{
    let seg_len = u32::from_le_bytes(iter.next_chunk::<4>().ok()?) as usize;
    let seg_data = iter.clone().take(seg_len).map(|x| x ^ salt).collect();
    iter.advance_by(seg_len).ok()?;
    Some(seg_data)
}

/// A wrapped function for reading data
/// +----------------------------------------------------------+
/// |                         segment                          |
/// +----------------------------+-----------------------------+
/// |        segment_len         |         segment_data        |
/// |  length_of_encrypted_data  |   encrypted_data_with_salt  |
/// +----------------------------+-----------------------------+
fn read_segment_reader<R: Read>(reader: &mut R, salt: u8) -> Option<Vec<u8>> {
    let mut seg_len: [u8; 4] = Default::default();
    reader.read_exact(&mut seg_len).ok()?;
    let seg_len = u32::from_le_bytes(seg_len);

    let mut seg_data: Vec<u8> = vec![0; seg_len as usize];
    reader.read_exact(&mut seg_data).ok()?;
    seg_data.iter_mut().for_each(|x| *x ^= salt);
    Some(seg_data)
}

/// Parse the ncm file with iterator. If you have an ncm file
/// stored in [Vec] or [slice](std::slice), you should use this.
/// # Example
/// ```
/// // Open file and store it in Vec.
/// let mut ncm_file = Vec::New();
/// std::fs::File::open("xxx.ncm").unwrap().read_to_end(&mut ncm_file).unwrap();
///
/// // Parse it with `from_iter`
/// let parsed_ncm_file = ncm_parser::from_iter(ncm_file.into_iter()).unwrap();
/// ```
pub fn from_iter<T>(mut iter: T) -> Result<NCMFile, ParseError>
where
    T: Iterator<Item = u8> + Clone,
{
    if iter.next_chunk::<10>().map_err(|_| EndOfFile)? != *b"CTENFDAM\x01\x70" {
        return Err(InvalidHeader);
    }
    let rc4_key = read_segment_iter(&mut iter, 0x64).ok_or(EndOfFile)?;
    let metadata = read_segment_iter(&mut iter, 0x63).ok_or(EndOfFile)?;
    let mut iter = iter.skip(9);
    let image = read_segment_iter(&mut iter, 0).ok_or(EndOfFile)?;
    let music = iter.collect();
    Ok(NCMFile {
        is_decrypted_flags: 0,
        rc4_key,
        metadata,
        image,
        music,
    })
}

/// Parse the ncm file with reader. If you have an ncm file
/// opened from [File](std::fs::File), you should use this.
/// # Example
/// ```
/// // Open file and parse it with `from_reader`
/// let parsed_ncm_file = ncm_parser::from_reader(std::fs::File::open("xxx.ncm").unwrap()).unwrap();
/// ```
pub fn from_reader<R: Read>(mut reader: R) -> Result<NCMFile, ParseError> {
    let mut ncm_header: [u8; 10] = Default::default();
    reader.read_exact(&mut ncm_header).map_err(|_| EndOfFile)?;
    if ncm_header != *b"CTENFDAM\x01\x70" {
        return Err(InvalidHeader);
    }
    let rc4_key = read_segment_reader(&mut reader, 0x64).ok_or(EndOfFile)?;
    let metadata = read_segment_reader(&mut reader, 0x63).ok_or(EndOfFile)?;
    reader.read_exact(&mut [0; 9]).map_err(|_| EndOfFile)?;
    let image = read_segment_reader(&mut reader, 0).ok_or(EndOfFile)?;
    let mut music = Vec::new();
    reader.read_to_end(&mut music).map_err(|_| EndOfFile)?;
    Ok(NCMFile {
        is_decrypted_flags: 0,
        rc4_key,
        metadata,
        image,
        music,
    })
}

/// A struct contains all the data parsed from the ncm file.
#[derive(Debug, Clone)]
pub struct NCMFile {
    is_decrypted_flags: u8,
    rc4_key: Vec<u8>,
    metadata: Vec<u8>,
    image: Vec<u8>,
    music: Vec<u8>,
}

impl NCMFile {
    /// Get music. Usually in MP3 or FLAC format.
    /// This function contains the decrypting precedure if calling the first time,
    /// and directly return the decrypted data after first-time calling.
    pub fn get_music(&mut self) -> Result<&Vec<u8>, ParseError> {
        if self.is_decrypted_flags & 0b0000_0001 != 0 {
            return Ok(&self.music);
        }
        // The music data is not decrypted now.
        self.is_decrypted_flags |= 0b0000_0001;

        // Decrypt RC4 key with AES-128
        let rc4_key = Aes128Dec::new(b"hzHRAmso5kInbaxW".into())
            .decrypt_padded::<Pkcs7>(&mut self.rc4_key)
            .map_err(|_| DecryptRC4KeyFailed)?;
        if !rc4_key.starts_with(b"neteasecloudmusic") {
            return Err(DecryptRC4KeyFailed);
        }
        let rc4_key = rc4_key[17..].iter().cycle();

        // Decrypt Music with modified Rivest Cipher 4
        // RC4-RSA
        let mut rc4_sbox: [u8; 256] = std::array::from_fn(|i| i as u8);

        let mut j: u8 = 0;
        for (i, key) in (0..=255).zip(rc4_key) {
            j = rc4_sbox[i].wrapping_add(j).wrapping_add(*key);
            rc4_sbox.swap(i as usize, j as usize);
        }

        // RC4-PRGA but no swap and iteration
        let out_stream = std::array::from_fn::<u8, 256, _>(|i| {
            // i as u8 as usize == i & 0xff
            // Would too many 'as' affect performance?
            let i = i + 1;
            let j = rc4_sbox[i as u8 as usize] as usize;
            let k = rc4_sbox[(i + j) as u8 as usize] as usize;
            return rc4_sbox[(j + k) as u8 as usize];
        })
        .into_iter()
        .cycle();

        // The compiler has done the SIMD optimization here.
        self.music
            .iter_mut()
            .zip(out_stream)
            .for_each(|(x, key)| *x ^= key);

        return Ok(&self.music);
    }

    /// Get cover image. Usually in PNG or JPEG format.
    /// Same as [`get_image_unchecked()`](NCMFile::get_image_unchecked()).
    pub fn get_image(&self) -> &Vec<u8> {
        &self.image
    }

    /// Get metadata.
    /// This function contains the decrypting precedure if calling the first time,
    /// and directly return the decrypted data after first-time calling.
    pub fn get_metadata(&mut self) -> Result<&Vec<u8>, ParseError> {
        if self.is_decrypted_flags & 0b0000_0010 != 0 {
            return Ok(&self.metadata);
        }
        // The metadata is not decrypted now.
        self.is_decrypted_flags |= 0b0000_0010;

        if !self.metadata.starts_with(b"163 key(Don't modify):") {
            return Err(DecryptMetadataFailed);
        }
        // Decrypt metadata with BASE64
        let mut metadata = base64dec
            .decode(&self.metadata[22..])
            .map_err(|_| DecryptMetadataFailed)?;
        // Decrypt metadata with AES-128
        let metadata = Aes128Dec::new(b"#14ljk_!\\]&0U<'(".into())
            .decrypt_padded::<Pkcs7>(&mut metadata)
            .map_err(|_| DecryptMetadataFailed)?;
        if !metadata.starts_with(b"music:") {
            return Err(DecryptMetadataFailed);
        }
        self.metadata = metadata[6..].to_vec();

        Ok(&self.metadata)
    }

    /// Directly get cover image. Usually in PNG or JPEG format.
    /// Same as [`get_image()`](NCMFile::get_image()).
    pub fn get_image_unchecked(&self) -> &Vec<u8> {
        &self.image
    }

    /// Directly get music.
    /// The music data is not decrypted if [`get_music()`](NCMFile::get_music()) has never been called.
    pub fn get_music_unchecked(&self) -> &Vec<u8> {
        &self.music
    }

    /// Directly get metadata.
    /// The metadata is not decrypted if [`get_metadata()`](NCMFile::get_metadata()) has never been called.
    pub fn get_metadata_unchecked(&self) -> &Vec<u8> {
        &self.metadata
    }

    #[cfg(feature = "serde_json")]
    /// Parse the JSON format metadata into struct.
    pub fn get_parsed_metadata(&mut self) -> Result<NCMMetadata, ParseError> {
        let metadata = self.get_metadata()?;
        #[allow(deprecated)]
        return NCMMetadata::new(metadata).ok_or(ParseMetadataFailed);
    }
}

#[cfg(feature = "serde_json")]
type Id = u64;

#[cfg(feature = "serde_json")]
type Name = String;

#[cfg(feature = "serde_json")]
#[derive(Debug, Clone)]
#[allow(missing_docs)]
/// A struct contains all the JSON values in metadata.
pub struct NCMMetadata {
    pub music: (Name, Id),
    pub artist: Vec<(Name, Id)>,
    pub album: (Name, Id),
    pub album_pic_doc_id: String,
    pub album_pic_url: String,
    pub bitrate: u32,
    pub mp3_doc_id: String,
    pub duration: u32,
    pub mv_id: u32,
    pub alias: Vec<String>,
    pub trans_names: Vec<String>,
    pub format: String,
}

#[cfg(feature = "serde_json")]
impl NCMMetadata {
    #[deprecated(since = "0.2.0", note = "Use `NCMFile::get_parsed_metadata()` instead.")]
    /// Parse the JSON format metadata into struct.
    /// Returns [`None`] if parsing failed.
    pub fn new(metadata: &[u8]) -> Option<Self> {
        let json: serde_json::Value = serde_json::from_slice(metadata).ok()?;
        let music_id = json["musicId"].as_u64()?;
        let music_name = json["musicName"].as_str()?.to_string();

        let artist = json["artist"]
            .as_array()?
            .into_iter()
            .map(|x| {
                let [ref name, ref id] = x.as_array()?[0..2] else {
                    return None;
                };
                Some((name.as_str()?.to_string(), id.as_u64()?))
            })
            .try_collect::<Vec<_>>()?;

        let album_id = json["albumId"].as_u64()?;
        let album = json["album"].as_str()?.to_string();
        let album_pic_doc_id = json["albumPicDocId"].as_str()?.to_string();
        let album_pic_url = json["albumPic"].as_str()?.to_string();
        let bitrate = json["bitrate"].as_u64()? as u32;
        let mp3_doc_id = json["mp3DocId"].as_str()?.to_string();
        let duration = json["duration"].as_u64()? as u32;
        let mv_id = json["mvId"].as_u64()? as u32;

        let alias = json["alias"]
            .as_array()?
            .into_iter()
            .map(|x| x.as_str().map(|x| x.to_string()))
            .try_collect::<Vec<_>>()?;

        let trans_names = json["transNames"]
            .as_array()?
            .into_iter()
            .map(|x| x.as_str().map(|x| x.to_string()))
            .try_collect::<Vec<_>>()?;

        let format = json["format"].as_str()?.to_string();

        Some(Self {
            music: (music_name, music_id),
            artist,
            album: (album, album_id),
            album_pic_doc_id,
            album_pic_url,
            bitrate,
            mp3_doc_id,
            duration,
            mv_id,
            alias,
            trans_names,
            format,
        })
    }
}
