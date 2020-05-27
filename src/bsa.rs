/* bas.rs */

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::str;

pub struct BSAFile {
    files: FileList,
    is_loaded: bool,
    filename: String,
    lookup: HashMap<String, u32>,
}

pub type FileList = Vec<FileStruct>;

fn calculate_hash(_name: &String) -> u64 {
    // let lower_name = name.to_ascii_lowercase();
    // let midpoint = lower_name.chars().count() >> 1;
    // let mut low = [0u8; 4];
    // let mut i = 0;
    // while i < midpoint {
    //     low[i & 3] ^= name.as_bytes()[i];
    //     i += 1;
    // }

    // let mut high = 0b00000000;
    // while i < name.len() {
    //     let temp = (name.as_bytes()[i] as u32) << (((i - midpoint) & 3) << 3);
    //     let bits = temp & 0x1F;
    //     high ^= temp;
    //     high = high << (32 - bits) | high >> bits;
    //     i += 1;
    // }
    // u32::from_le_bytes(low) as u64 | (high as u64) << 32
    0
}

impl BSAFile {
    const MAGIC_HEADER: &'static [u8] = &[0x0, 0x1, 0x0, 0x0];

    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            is_loaded: false,
            filename: String::new(),
            lookup: HashMap::new(),
        }
    }

    pub fn open(&mut self, file: String) {
        self.filename = file;
        self.read_header()
    }

    pub fn exists(&self, file: &String) -> bool {
        self.get_index(file) != -1
    }

    pub fn get_file(&self, file: &String) -> Vec<u8> {
        let i = self.get_index(file);
        if i == -1 {
            let msg = format!("File not found: {}", file);
            self.fail(msg.as_str());
        }
        let fs = &self.files[i as usize];

        let mut file = File::open(&self.filename).unwrap();
        file.seek(SeekFrom::Start(fs.offset as u64)).unwrap();
        let mut buf = vec![0u8; fs.file_size as usize];
        file.read_exact(&mut buf).unwrap();
        buf
    }

    pub fn get_list(&self) -> &FileList {
        &self.files
    }

    fn fail(&self, msg: &str) {
        panic!(
            "BSA Error: {}
Archive: {}",
            msg, self.filename
        )
    }

    fn read_header(&mut self) {
        assert_eq!(self.is_loaded, false);

        let mut file = File::open(&self.filename).unwrap();

        // Total archive size
        let fsize = file.seek(SeekFrom::End(0)).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        if fsize < 12 {
            self.fail("File too small to be a valid BSA archive")
        }

        // Get essential header numbers
        let dirsize: u32;
        let filenum: u32;
        {
            // First 12 bytes
            let mut buff = [0u8; 4];
            file.read(&mut buff).unwrap();

            if buff[..4] != *BSAFile::MAGIC_HEADER {
                self.fail("Unrecognized BSA header")
            }

            // Total number of bytes used in size/offset-table + filename
            // sections. AKA hashOffset.
            file.read(&mut buff).unwrap();
            dirsize = u32::from_le_bytes(buff);

            // Number of files
            file.read(&mut buff).unwrap();
            filenum = u32::from_le_bytes(buff);
        }

        // Each file must take up at least 21 bytes of data in the bsa. So
        // if files*21 overflows the file size then we are guaranteed that
        // the archive is corrupt.
        if (filenum as u64 * 21 > (fsize - 12))
            || (dirsize as u64 + 8 * filenum as u64 > (fsize - 12))
        {
            self.fail("Directory information larger than entire archive");
        }

        // Read the offset info into a temporary buffer
        let mut offsets: Vec<u32> = vec![0; 3 * filenum as usize];
        for i in 0..3 * filenum {
            let mut buff = [0u8; 4];
            file.read(&mut buff).unwrap();
            offsets[i as usize] = u32::from_le_bytes(buff);
        }

        // Read the string table
        let mut buff: Vec<u8> = vec![0; dirsize as usize - 12 * filenum as usize]; //dirsize as usize - 12 * filenum as usize];
        file.read(&mut buff).unwrap();
        let string_buf = String::from_utf8(buff).unwrap();
        let string_vec = string_buf.split('\0').collect::<Vec<&str>>();

        // Check our position
        assert_eq!(
            file.seek(SeekFrom::Current(0)).unwrap(),
            12 + dirsize as u64
        );

        // Calculate the offset of the data buffer. All file offsets are
        // relative to this. 12 header bytes + directory + hash table (skipped)
        let file_data_offset = 12 + dirsize + 8 * filenum;

        // Set up the the FileStruct table
        for i in 0..filenum {
            let fs = FileStruct {
                file_size: offsets[i as usize * 2],
                offset: offsets[i as usize * 2 + 1] + file_data_offset,
                name: string_vec[i as usize].to_string(),
            };

            if fs.offset as u64 + fs.file_size as u64 > fsize {
                self.fail("Archive contains offsets outside itself")
            }
            self.lookup.insert(fs.name.to_string(), i);
            self.files.push(fs);
        }

        self.is_loaded = true;
    }

    pub fn create(&mut self, file: &String, filenames: &Vec<String>) {
        assert_eq!(self.is_loaded, false);
        self.filename = file.to_string();

        // track bytes written
        let mut bytes_written: u32 = 0;
        // get file count
        let filenum = filenames.len() as u32;
        // get all file sizes
        let mut total_files_size: u32 = 0;
        for (i, filename) in filenames.iter().enumerate() {
            let archive_path = filename.to_ascii_lowercase().replace("/", "\\");
            let metadata = fs::metadata(filename).unwrap();
            let fsize = metadata.len();
            // check size
            let fs = FileStruct {
                file_size: fsize as u32,
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
        bytes_written += f.write(BSAFile::MAGIC_HEADER).unwrap() as u32;
        // write hashOffset
        // Offset of the hash table in the file, minus the header size (12)
        // calculate from 12*numfiles + length of each file name null-terminated
        let mut hash_offset: u32 = 12 * filenum;
        for file in &self.files {
            hash_offset += file.name.chars().count() as u32 + 1;
        }
        // hash_offset -= 12;
        // hash_offset += 1;
        bytes_written += f.write(&hash_offset.to_le_bytes()).unwrap() as u32;
        // write fileCount
        bytes_written += f.write(&filenum.to_le_bytes()).unwrap() as u32;
        assert_eq!(bytes_written, 12);

        // write sizes/offsets
        for file in &self.files {
            // file size
            bytes_written += f.write(&file.file_size.to_le_bytes()).unwrap() as u32;
            // offset of file in the data section
            bytes_written += f.write(&file.offset.to_le_bytes()).unwrap() as u32;
        }

        // write filename offsets
        let mut starting_offset: u32 = 0;
        for file in &self.files {
            // Relative offset of the filename in the records section
            bytes_written += f.write(&starting_offset.to_le_bytes()).unwrap() as u32;
            let mut filename_length = file.name.chars().count() as u32;
            filename_length += 1; // null terminator
            starting_offset += filename_length;
        }
        assert_eq!(bytes_written, 12 + 12 * filenum);

        // write filesnames
        let null_term = ['\0' as u8];
        for file in &self.files {
            bytes_written += f.write(file.name.as_bytes()).unwrap() as u32;
            bytes_written += f.write(&null_term).unwrap() as u32;
        }

        // write hash table block
        for file in &self.files {
            let terminated = file.name.to_string() + "\0";
            let hash = calculate_hash(&terminated);
            bytes_written += f.write(&hash.to_le_bytes()).unwrap() as u32;
        }

        // write files
        for (i, filename) in filenames.iter().enumerate() {
            let mut read_buf: Vec<u8> = Vec::new();

            // read in the file data
            let rfile = File::open(filename).unwrap();
            let mut reader = BufReader::new(rfile);
            reader.read_to_end(&mut read_buf).unwrap();

            assert_eq!(self.files.get(i).unwrap().file_size, read_buf.len() as u32);

            // write out the file data to the archive
            f.write_all(&read_buf).unwrap();
        }
        f.flush().unwrap();
    }

    // Get the index of a given file name, or -1 if not found
    fn get_index(&self, file: &String) -> i32 {
        match self.lookup.get(file) {
            Some(&index) => index as i32,
            _ => -1,
        }
    }
}

pub struct FileStruct {
    pub file_size: u32,
    pub offset: u32,
    pub name: String,
}
