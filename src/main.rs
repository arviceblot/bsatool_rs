use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use indicatif::ProgressBar;
use tabled::builder::Builder;
use tabled::settings::Style;

use bsatoollib as bsa;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// BSA file to use
    file: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List the files presents in the given BSA file
    List {
        /// Include extra information in archive listing
        #[arg(short, long, default_value_t = false)]
        long: bool,
    },
    /// Extract a file from the given BSA file
    Extract {
        /// Create directory hierarchy on file extraction
        #[arg(short, long, default_value_t = true)]
        full_path: bool,
        /// Output directory path
        #[arg(short, long, default_value_t = String::from("."), value_hint = clap::ValueHint::DirPath)]
        output: String,
        /// Files to extract from BSA
        #[arg(short, long)]
        extract_files: Vec<String>,
    },
    /// Extract all files from the given BSA file
    ExtractAll {
        /// Output directory path
        #[arg(short, long, default_value_t = String::from("."), value_hint = clap::ValueHint::DirPath)]
        output: String,
    },
    /// Create a new BSA file with given files for archiving
    Create {
        /// Files to add to BSA
        #[arg(short, long)]
        files: Vec<String>,
    },
}

fn list(bsa: bsa::BSAFile, long_format: bool) {
    let files = bsa.get_list();

    if long_format {
        for file in files {
            println!("{}", file.name)
        }
        return;
    }

    // longformat
    let mut builder = Builder::default();
    builder.set_header(["name", "size", "offset"]);
    for file in files {
        builder.push_record([
            file.name.to_string(),
            file.file_size.to_string(),
            format!("0x{:x}", file.offset),
        ]);
    }
    let mut table = builder.build();
    table.with(Style::modern());
    println!("{}", table);
}

fn extract(
    bsa: bsa::BSAFile,
    full_path: bool,
    out_dir: &String,
    extract_files: &[String],
) -> Result<()> {
    for extract_file in extract_files.iter() {
        let archive_path = extract_file.replace('/', "\\");
        let extract_path = extract_file.replace('\\', "/");

        if !bsa.exists(&archive_path) {
            println!("ERROR: file '{}' not found in BSA!", extract_file);
            continue;
        }

        let rel_path = Path::new(&extract_path);
        let mut target = PathBuf::from(&out_dir);
        if full_path {
            target.push(rel_path);
        } else {
            target.push(rel_path.file_name().unwrap());
        }

        // Create the directory hierarchy
        fs::create_dir_all(target.parent().unwrap())?;

        if !target.parent().unwrap().is_dir() {
            panic!(
                "ERROR: {} is not a directory.",
                target.parent().unwrap().to_str().unwrap()
            );
        }

        // Get a buffer for the file to extract
        let data = bsa.get_file(&archive_path)?;

        // Write the file to disk
        println!(
            "Extracting {} to {}",
            extract_file,
            target.to_str().unwrap()
        );
        let f = File::create(target).expect("Unable to create file");
        let mut f = BufWriter::new(f);
        f.write_all(&data)?;
        f.flush()?;
    }
    Ok(())
}

fn extract_all(bsa: bsa::BSAFile, out_dir: &String) -> Result<()> {
    // Get the list of files present in the archive
    let list = bsa.get_list();
    let pb = ProgressBar::new(list.len() as u64);

    for file in list {
        pb.inc(1);
        let extract_path = file.name.replace('\\', "/");

        // Get the target path (the path the file will be extracted to)
        let target = Path::new(&out_dir).join(extract_path);

        // Create the directory hierarchy
        fs::create_dir_all(target.parent().unwrap())?;

        if !target.parent().unwrap().is_dir() {
            panic!(
                "ERROR: {} is not a directory.",
                target.parent().unwrap().to_str().unwrap()
            );
        }

        // Get a buffer for the file to extract
        let data = bsa.get_file(&file.name)?;

        // Write the file to disk
        let f = File::create(target).expect("Unable to create file");
        let mut f = BufWriter::new(f);
        f.write_all(&data)?;
        f.flush()?;
    }
    pb.finish_with_message("done");
    Ok(())
}

fn main() {
    let args = Cli::parse();

    // Open file
    let mut bsa: bsa::BSAFile = bsa::BSAFile::default();
    let filename = args.file;
    match &args.command {
        Commands::List { long } => {
            bsa.open(&filename).unwrap();
            list(bsa, *long);
        }
        Commands::Extract {
            full_path,
            output,
            extract_files,
        } => {
            bsa.open(&filename).unwrap();
            extract(bsa, *full_path, output, extract_files).unwrap();
        }
        Commands::ExtractAll { output } => {
            bsa.open(&filename).unwrap();
            extract_all(bsa, output).unwrap();
        }
        Commands::Create { files } => {
            bsa.create(&filename, files).unwrap();
        }
    }
}
