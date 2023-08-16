//! A simple library to using Bethesda BSA files.
//!
//! # Usage
//!
//! ```rust
//! use bsatoollib as bsa;
//!
//! // open an existing BSA file
//! let bsa = bsa::BSAFile::new("SomeFile.BSA").unwrap();
//!
//! // print all file names in the BSA
//! for file in bsa.get_list().iter() {
//!     println!(file.name);
//! }
//! ```
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
pub mod error; // expose for result matching

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::str;

use error::{BsaError, Result};

const MAGIC_HEADER: &[u8] = &[0x0, 0x1, 0x0, 0x0];

/// Vec of FileStruct type
pub type FileList = Vec<FileStruct>;

fn calculate_hash(name: &str) -> u64 {
    let lower_name = name.to_ascii_lowercase();
    let characters: Vec<char> = lower_name.chars().collect();
    let l = lower_name.chars().count() as u32 >> 1;

    let (mut sum, mut off): (u32, u32) = (0, 0);
    for (i, c) in characters.iter().enumerate() {
        if i as u32 >= l {
            break;
        }
        sum ^= (*c as u32) << (off & 0x1F);
        off += 8;
    }
    let low = sum;

    let mut sum: u64 = 0;
    off = 0;
    let (mut temp, mut n);
    for c in characters.iter() {
        temp = (*c as u32) << (off & 0x1F);
        sum ^= temp as u64;
        n = temp & 0x1F;
        sum = (sum << (32 - n)) | (sum >> n); // binary "rotate right"
        off += 8;
    }
    let high = sum;
    (low as u64) | (high << 32)
}

fn check_bytes_written(expected: u32, actual: usize) -> Result<()> {
    if expected != actual as u32 {
        return Err(BsaError::BytesWritten { expected, actual });
    }
    Ok(())
}

/// Helper data struct for storing info related to a file with a BSA
#[derive(Debug)]
pub struct FileStruct {
    /// Expected size of the file in bytes
    pub file_size: usize,
    /// Offset of the file in bytes from the start of the BSA
    pub offset: u32,
    /// Name of the file
    pub name: String,
}

/// Main struct for reading and manipulating BSAs
#[derive(Debug, Default)]
pub struct BSAFile<'a> {
    files: FileList,
    is_loaded: bool,
    filename: &'a str,
    lookup: HashMap<String, u32>,
}

impl<'a> BSAFile<'a> {
    /// Create a new BSAFile object and open a given file
    pub fn new(file: &'a str) -> Result<Self> {
        let mut obj = Self::default();
        obj.open(file)?;
        Ok(obj)
    }

    /// Open a BSA file for reading
    pub fn open(&mut self, file: &'a str) -> Result<()> {
        // clear out any existing file data
        self.filename = file;
        self.files.clear();
        self.is_loaded = false;
        self.lookup.clear();

        // read BSA header
        self.read_header()
    }

    /// Check whether a given file name exists within the BSA
    pub fn exists(&self, file: &str) -> bool {
        self.ensure_loaded().unwrap();
        self.get_index(file).is_ok()
    }

    /// Get the file bytes for a given file name within the BSA
    pub fn get_file(&self, file: &str) -> Result<Vec<u8>> {
        self.ensure_loaded()?;
        let i = self.get_index(file)?;
        let fs = &self.files[i as usize];

        let mut file = File::open(self.filename)?;
        file.seek(SeekFrom::Start(fs.offset as u64))?;
        let mut buf = vec![0u8; fs.file_size];
        file.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Get the data for files with the BSA
    pub fn get_list(&self) -> &FileList {
        self.ensure_loaded().unwrap();
        &self.files
    }

    fn ensure_loaded(&self) -> Result<()> {
        if !self.is_loaded {
            return Err(BsaError::NotOpen);
        }
        Ok(())
    }

    fn ensure_not_loaded(&self) -> Result<()> {
        if self.is_loaded {
            return Err(BsaError::AlreadyOpen);
        }
        Ok(())
    }

    fn read_header(&mut self) -> Result<()> {
        self.ensure_not_loaded()?;
        let mut file = File::open(self.filename)?;

        // Total archive size
        let fsize = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;
        if fsize < 12 {
            return Err(BsaError::TooSmall(fsize));
        }

        // Get essential header numbers
        let dirsize: u32;
        let filenum: u32;
        {
            // First 12 bytes
            let mut buff = [0u8; 4];
            file.read_exact(&mut buff)?;

            if buff[..4] != *MAGIC_HEADER {
                return Err(BsaError::BadHeader);
            }

            // Total number of bytes used in size/offset-table + filename
            // sections. AKA hashOffset.
            file.read_exact(&mut buff)?;
            dirsize = u32::from_le_bytes(buff);

            // Number of files
            file.read_exact(&mut buff)?;
            filenum = u32::from_le_bytes(buff);
        }

        // Each file must take up at least 21 bytes of data in the bsa. So
        // if files*21 overflows the file size then we are guaranteed that
        // the archive is corrupt.
        if (filenum as u64 * 21 > (fsize - 12))
            || (dirsize as u64 + 8 * filenum as u64 > (fsize - 12))
        {
            return Err(BsaError::DirSize);
        }

        // Read the offset info into a temporary buffer
        let mut offsets: Vec<u32> = Vec::with_capacity(filenum as usize);
        let mut offsets_handle = file.take(12 * filenum as u64);
        let mut offsets_buffer = Vec::new();
        offsets_handle.read_to_end(&mut offsets_buffer)?;
        let mut buff: [u8; 4];
        for b in (0..12 * filenum as usize).step_by(4) {
            buff = offsets_buffer[b..b + 4].try_into().unwrap();
            offsets.push(u32::from_le_bytes(buff));
        }
        let mut file = offsets_handle.get_ref();

        // Read the string table
        let mut buff: Vec<u8> = vec![0; dirsize as usize - 12 * filenum as usize]; //dirsize as usize - 12 * filenum as usize];
        file.read_exact(&mut buff)?;
        let string_buf = String::from_utf8(buff).unwrap();
        let string_vec = string_buf.split('\0').collect::<Vec<&str>>();

        // Check our position
        if file.stream_position()? != 12 + dirsize as u64 {
            return Err(BsaError::Position {
                expected: 12 + dirsize,
                actual: file.stream_position()?,
            });
        }

        // Calculate the offset of the data buffer. All file offsets are
        // relative to this. 12 header bytes + directory + hash table (skipped)
        let file_data_offset = 12 + dirsize + 8 * filenum;

        // Set up the the FileStruct table
        for i in 0..filenum {
            let fs = FileStruct {
                file_size: offsets[i as usize * 2] as usize,
                offset: offsets[i as usize * 2 + 1] + file_data_offset,
                name: string_vec[i as usize].to_string(),
            };

            if fs.offset as u64 + fs.file_size as u64 > fsize {
                return Err(BsaError::OffsetOutside);
            }
            self.lookup.insert(fs.name.to_string(), i);
            self.files.push(fs);
        }

        self.is_loaded = true;

        Ok(())
    }

    /// Create a new BSA file, populating it with files from given file names
    pub fn create(&mut self, file: &'a str, filenames: &[String]) -> Result<()> {
        self.ensure_not_loaded()?;
        self.filename = file;

        // track bytes written
        let mut bytes_written: usize = 0;
        // get file count
        let filenum = filenames.len() as u32;
        // get all file sizes
        let mut total_files_size: u32 = 0;
        for (i, filename) in filenames.iter().enumerate() {
            let archive_path = filename.to_ascii_lowercase().replace('/', "\\");
            let metadata = fs::metadata(filename)?;
            let fsize = metadata.len();
            // check size
            let fs = FileStruct {
                file_size: fsize as usize,
                name: archive_path,
                offset: total_files_size,
            };
            total_files_size += fsize as u32;

            self.lookup.insert(fs.name.to_string(), i as u32);
            self.files.push(fs);
        }

        // build header
        let f = File::create(file).expect("Unable to create file");
        let mut f = BufWriter::new(f);
        // write magic header
        bytes_written += f.write(MAGIC_HEADER)?;
        // write hashOffset
        // Offset of the hash table in the file, minus the header size (12)
        // calculate from 12*numfiles + length of each file name null-terminated
        let mut hash_offset: u32 = 12 * filenum;
        for file in &self.files {
            hash_offset += file.name.chars().count() as u32 + 1;
        }
        // hash_offset -= 12;
        // hash_offset += 1;
        bytes_written += f.write(&hash_offset.to_le_bytes())?;
        // write fileCount
        bytes_written += f.write(&filenum.to_le_bytes())?;
        check_bytes_written(12, bytes_written)?;

        // write sizes/offsets
        for file in &self.files {
            // file size
            bytes_written += f.write(&file.file_size.to_le_bytes())?;
            // offset of file in the data section
            bytes_written += f.write(&file.offset.to_le_bytes())?;
        }

        // write filename offsets
        let mut starting_offset: u32 = 0;
        for file in &self.files {
            // Relative offset of the filename in the records section
            bytes_written += f.write(&starting_offset.to_le_bytes())?;
            let mut filename_length = file.name.chars().count() as u32;
            filename_length += 1; // null terminator
            starting_offset += filename_length;
        }
        check_bytes_written(12 + 12 * filenum, bytes_written)?;

        // write filesnames
        let null_term = [b'\0'];
        for file in &self.files {
            bytes_written += f.write(file.name.as_bytes())?;
            bytes_written += f.write(&null_term)?;
        }

        // write hash table block
        for file in &self.files {
            let terminated = file.name.to_string() + "\0";
            let hash = calculate_hash(&terminated);
            bytes_written += f.write(&hash.to_le_bytes())?;
        }

        // write files
        for (i, filename) in filenames.iter().enumerate() {
            let mut read_buf: Vec<u8> = Vec::new();

            // read in the file data
            let rfile = File::open(filename)?;
            let mut reader = BufReader::new(rfile);
            reader.read_to_end(&mut read_buf)?;

            check_bytes_written(read_buf.len() as u32, self.files.get(i).unwrap().file_size)?;

            // write out the file data to the archive
            f.write_all(&read_buf)?;
        }
        f.flush()?;
        Ok(())
    }

    // Get the index of a given file name, or -1 if not found
    fn get_index(&self, file: &str) -> Result<u32> {
        match self.lookup.get(file) {
            Some(&index) => Ok(index),
            None => Err(BsaError::FileNotFound(file.to_string())),
        }
    }
}
