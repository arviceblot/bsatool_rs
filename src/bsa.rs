/* bas.rs */

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::str;

pub struct BSAFile {
    files: FileList,
    string_buf: String,
    is_loaded: bool,
    filename: String,
    lookup: HashMap<String, u32>,
}

pub type FileList = Vec<FileStruct>;

impl BSAFile {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            string_buf: String::new(),
            is_loaded: false,
            filename: String::new(),
            lookup: HashMap::new(),
        }
    }
    pub fn open(&mut self, file: String) {
        self.filename = file;
        self.read_header()
    }
    pub fn exists(&self, file: String) -> bool {
        self.get_index(file) != -1
    }
    pub fn get_file(&self, file: String) -> Vec<u8> {
        let i = self.get_index(file.to_string());
        if i == -1 {
            let msg = format!("File not found: {}", file.to_string());
            self.fail(msg);
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

    fn fail(&self, msg: String) {
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
            self.fail("File too small to be a valid BSA archive".to_string())
        }

        // Get essential header numbers
        let dirsize: u32;
        let filenum: u32;
        {
            // First 12 bytes
            let mut buff = [0u8; 4];
            file.read(&mut buff).unwrap();

            if buff[..4] != [0x0, 0x1, 0x0, 0x0] {
                self.fail("Unrecognized BSA header".to_string())
            }

            // Total number of bytes used in size/offset-table + filename
            // sections.
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
            self.fail("Directory information larger than entire archive".to_string());
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
        self.string_buf = String::with_capacity(dirsize as usize - 12 * filenum as usize);
        file.read(&mut buff).unwrap();
        self.string_buf = String::from_utf8(buff).unwrap();
        let string_vec = self.string_buf.split('\0').collect::<Vec<&str>>();

        // Check our position
        assert_eq!(
            file.seek(SeekFrom::Current(0)).unwrap(),
            12 + dirsize as u64
        );

        // Calculate the offset of the data buffer. All file offsets are
        // relative to this. 12 header bytes + directory + hash table
        // (skipped)
        let file_data_offset = 12 + dirsize + 8 * filenum;

        // Set up the the FileStruct table
        for i in 0..filenum {
            let fs = FileStruct {
                file_size: offsets[i as usize * 2],
                offset: offsets[i as usize * 2 + 1] + file_data_offset,
                name: string_vec[i as usize].to_string(),
            };

            if fs.offset as u64 + fs.file_size as u64 > fsize {
                self.fail("Archive contains offsets outside itself".to_string())
            }
            self.lookup.insert(fs.name.to_string(), i);
            self.files.push(fs);
        }

        self.is_loaded = true;
    }
    // Get the index of a given file name, or -1 if not found
    fn get_index(&self, file: String) -> i32 {
        let i = self.files.iter().position(|x| x.name == file).unwrap();
        i as i32
    }
}

pub struct FileStruct {
    pub file_size: u32,
    pub offset: u32,
    pub name: String,
}
