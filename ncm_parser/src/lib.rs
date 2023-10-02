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
//!     let ncm_file_name = Path::new("xxx.ncm");
//!     let mut ncm_file = std::fs::read(ncm_file_name).unwrap();
//!
//!     // Parse ncm file with `from_iter`
//!     let mut ncm_file_from_iter =
//!         ncm_parser::from_iter(ncm_file.into_iter()).unwrap();
//!
//!     // Directly parse ncm file with `from_reader`
//!     let mut ncm_file_from_reader =
//!         ncm_parser::from_reader(File::open(ncm_file_name).unwrap()).unwrap();
//!
//!     // Both functions get same result.
//!     assert_eq!(
//!         ncm_file_from_iter.get_image().unwrap(),
//!         ncm_file_from_reader.get_image().unwrap()
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
//!     // Parse metadata
//!     let ncm_meta = ncm_file_from_iter.get_parsed_metadata().unwrap();
//!
//!     let image = ncm_file_from_iter.get_image_unchecked();
//!     let metadata = ncm_file_from_iter.get_metadata_unchecked();
//!     let music = ncm_file_from_iter.get_music_unchecked();
//!
//!     // Save music
//!     let music_name = ncm_file_name.with_extension(&ncm_meta.format);
//!     std::fs::write(music_name, &music).unwrap();
//!
//!     // Get image format
//!     let image_ext = ncm_meta.album_pic_url.rsplit_once('.').unwrap().1;
//!
//!     // Save cover image
//!     let image_name = ncm_file_name.with_extension(image_ext);
//!     std::fs::write(image_name, &image).unwrap();
//!
//!     // Save metadata
//!     let meta_name = ncm_file_name.with_extension("json");
//!     std::fs::write(meta_name, &metadata).unwrap();
//! }
//! ```

#![feature(never_type)]
#![feature(iter_next_chunk)]
#![feature(iter_advance_by)]
#![feature(iterator_try_collect)]
#![feature(doc_auto_cfg)]
#![warn(missing_docs)]
use std::io::Read;

use aes::Aes128Dec;
use base64::engine::general_purpose::STANDARD as base64dec;
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
    /// The ncm file header does not match \"CTENFDAM\",
    /// which indicates the input file may not be ncm format.
    InvalidHeader,
    #[error("Failed to decrypt ncm RC4 key.")]
    /// Failed to decrypt the AES-128 encrypted RC4 key.
    /// We can't get the music data without correctly decrypted RC4 key.
    DecryptRC4KeyFailed,
    #[error("Failed to decrypt ncm metadata.")]
    /// Failed to decrypt the AES-128 and BASE64 encrypted matadata.
    DecryptMetadataFailed,
    #[error("Failed parsing ncm metadata. [{0}]")]
    /// Failed to parse the JSON format metadata into struct.
    ParseMetadataFailed(&'static str),
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

/// Parse the ncm file with iterator. Recommended if you have an ncm file
/// stored in [Vec] or [slice](std::slice).
/// # Example
/// ```
/// // Open file and store it in Vec.
/// let mut ncm_file = std::fs::read("xxx.ncm").unwrap();
///
/// // Parse it with `from_iter`
/// let parsed_ncm_file = ncm_parser::from_iter(ncm_file.into_iter()).unwrap();
/// ```
pub fn from_iter<T>(mut iter: T) -> Result<NCMFile, ParseError>
where
    T: Iterator<Item = u8> + Clone,
{
    if iter.next_chunk::<10>().map_err(|_| EndOfFile)?[0..8] != *b"CTENFDAM" {
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

/// Parse the ncm file with reader. Recommended if you have an ncm file
/// opened from [File](std::fs::File).
/// # Example
/// ```
/// // Open file and parse it with `from_reader`
/// let parsed_ncm_file = ncm_parser::from_reader(std::fs::File::open("xxx.ncm").unwrap()).unwrap();
/// ```
pub fn from_reader<R: Read>(mut reader: R) -> Result<NCMFile, ParseError> {
    let mut ncm_header: [u8; 10] = Default::default();
    reader.read_exact(&mut ncm_header).map_err(|_| EndOfFile)?;
    if ncm_header[0..8] != *b"CTENFDAM" {
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
    pub fn get_image(&self) -> Result<&Vec<u8>, !> {
        Ok(&self.image)
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
        return NCMMetadata::new(metadata);
    }
}

#[cfg(feature = "serde_json")]
#[derive(Debug, Clone)]
#[allow(missing_docs)]
/// A struct contains all the JSON values in metadata.
pub struct NCMMetadata {
    /// music_id might not be a number.
    pub music_id: String,
    pub music_name: String,
    pub artists: Vec<(String, u64)>,
    pub album_id: u64,
    pub album_name: String,
    pub album_pic_doc_id: u64,
    pub album_pic_url: String,
    pub bitrate: u64,
    pub mp3_doc_id: Option<String>,
    pub duration: u64,
    pub mv_id: u64,
    pub alias: Vec<String>,
    pub trans_names: Vec<String>,
    pub format: String,
    pub fee: Option<u64>,
    pub flag: Option<u64>,
}

#[cfg(feature = "serde_json")]
impl NCMMetadata {
    #[deprecated(
        since = "0.2.0",
        note = "Use `NCMFile::get_parsed_metadata()` instead."
    )]
    /// Parse the JSON format metadata into struct.
    /// Returns [`None`] if parsing failed.
    pub fn new(metadata: &[u8]) -> Result<Self, ParseError> {
        use std::str::FromStr;

        let json: serde_json::Value = serde_json::from_slice(metadata)
            .map_err(|_| ParseMetadataFailed("Cannot read the ncm metadata."))?;

        let music_id = json["musicId"]
            .as_str()
            .map(|x| x.to_string())
            .unwrap_or_else(|| json["musicId"].to_string());

        let music_name = json["musicName"]
            .as_str()
            .ok_or(ParseMetadataFailed("Failed parsing [musicName]."))?
            .to_string();

        let artists: Vec<_> = json["artist"]
            .as_array()
            .ok_or(ParseMetadataFailed("Failed parsing [artist]."))?
            .into_iter()
            .map(|artist| {
                let [name, id] = &artist.as_array()?[0..2] else {
                    return None;
                };
                let name = name.as_str()?.to_string();
                let id = id.as_u64().or_else(|| u64::from_str(id.as_str()?).ok())?;
                return Some((name, id));
            })
            .try_collect()
            .ok_or(ParseMetadataFailed("Failed parsing [artist]."))?;

        let album_id = json["albumId"]
            .as_u64()
            .or_else(|| u64::from_str(json["albumId"].as_str()?).ok())
            .ok_or(ParseMetadataFailed("Failed parsing [albumId]."))?;

        let album_name = json["album"]
            .as_str()
            .ok_or(ParseMetadataFailed("Failed parsing [album]."))?
            .to_string();

        let album_pic_doc_id = json["albumPicDocId"]
            .as_u64()
            .or_else(|| u64::from_str(json["albumPicDocId"].as_str()?).ok())
            .ok_or(ParseMetadataFailed("Failed parsing [albumPicDocId]."))?;

        let album_pic_url = json["albumPic"]
            .as_str()
            .ok_or(ParseMetadataFailed("Failed parsing [albumPic]."))?
            .to_string();

        let bitrate = json["bitrate"]
            .as_u64()
            .ok_or(ParseMetadataFailed("Failed parsing [bitrate]."))?;

        let mp3_doc_id = json["mp3DocId"].as_str().map(|x| x.to_string());

        let duration = json["duration"]
            .as_u64()
            .ok_or(ParseMetadataFailed("Failed parsing [duration]."))?;

        let mv_id = json["mvId"]
            .as_u64()
            .or_else(|| u64::from_str(json["mvId"].as_str()?).ok())
            .unwrap_or_default();

        let alias: Vec<_> = json["alias"]
            .as_array()
            .ok_or(ParseMetadataFailed("Failed parsing [alias]."))?
            .into_iter()
            .map(|x| x.as_str().map(|x| x.to_string()))
            .try_collect()
            .ok_or(ParseMetadataFailed("Failed parsing [alias]."))?;

        let trans_names: Vec<_> = json["transNames"]
            .as_array()
            .ok_or(ParseMetadataFailed("Failed parsing [transNames]."))?
            .into_iter()
            .map(|x| x.as_str().map(|x| x.to_string()))
            .try_collect()
            .ok_or(ParseMetadataFailed("Failed parsing [transNames]."))?;

        let format = json["format"]
            .as_str()
            .ok_or(ParseMetadataFailed("Failed parsing [format]."))?
            .to_string();

        let fee = json["fee"].as_u64();

        let mut flag = json["flag"].as_u64();
        if flag == None {
            let privilege = json["privilege"].as_object();
            if let Some(inner_flag) = privilege {
                flag = inner_flag["flag"].as_u64();
            }
        }

        return Ok(Self {
            music_name,
            music_id,
            artists,
            album_name,
            album_id,
            album_pic_doc_id,
            album_pic_url,
            bitrate,
            mp3_doc_id,
            duration,
            mv_id,
            alias,
            trans_names,
            format,
            fee,
            flag,
        });
    }
}
