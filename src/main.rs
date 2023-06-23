use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Arg, Command};
use indicatif::ProgressBar;
use tabled::builder::Builder;

use bsatoollib as bsa;
use tabled::settings::Style;

struct Arguments {
    mode: String,
    filename: String,
    extractfile: String,
    outdir: String,
    filenames: Vec<String>,

    longformat: bool,
    fullpath: bool,
}

impl Arguments {
    pub fn new() -> Self {
        Self {
            mode: String::from("list"),
            filename: String::from(""),
            extractfile: String::from(""),
            outdir: String::from("."),
            longformat: false,
            fullpath: false,
            filenames: vec![],
        }
    }
}

fn parse_options() -> Arguments {
    let mut info = Arguments::new();

    let matches = Command::new("bsatool_rs")
        .about("Inspect and extract files from Bethesda BSA archives")
        .arg(
            Arg::new("INPUT")
                .required(true)
                .help("The input archive file to use"),
        )
        .subcommand(
            Command::new("list")
                .about("List the files presents in the input archive")
                .arg(
                    Arg::new("long")
                        .short('l')
                        .long("long")
                        .help("Include extra information in archive listing"),
                ),
        )
        .subcommand(
            Command::new("extract")
                .about("Extract a file from the input archive")
                .arg(Arg::new("full-path").short('f').long("full-path").help(
                    "Create directory hierarchy on file extraction (always true for extractall)",
                ))
                .arg(Arg::new("file_to_extract"))
                .arg(Arg::new("output_directory")),
        )
        .subcommand(
            Command::new("extractall")
                .about("Extract all files from the input archive")
                .arg(Arg::new("output_directory")),
        )
        .subcommand(
            Command::new("create")
                .about("Create an archive file")
                .arg(Arg::new("files").takes_value(true).multiple_values(true)),
        )
        .get_matches();

    info.filename = matches.value_of("INPUT").unwrap().to_string();

    if let Some(matches) = matches.subcommand_matches("list") {
        info.mode = String::from("list");
        info.longformat = matches.is_present("long");
    } else if let Some(matches) = matches.subcommand_matches("extract") {
        info.mode = String::from("extract");
        info.fullpath = matches.is_present("full-path");
        info.extractfile = matches
            .value_of("file_to_extract")
            .unwrap_or("")
            .to_string();
        info.outdir = matches
            .value_of("output_directory")
            .unwrap_or(".")
            .to_string();
    } else if let Some(matches) = matches.subcommand_matches("extractall") {
        info.mode = String::from("extractall");
        info.outdir = matches
            .value_of("output_directory")
            .unwrap_or(".")
            .to_string();
    } else if let Some(matches) = matches.subcommand_matches("create") {
        info.mode = String::from("create");
        info.filenames = matches
            .values_of("files")
            .unwrap()
            .map(|x| x.to_string())
            .collect();
    }
    info
}

fn list(bsa: &bsa::BSAFile, info: &Arguments) {
    let files = bsa.get_list();

    if !info.longformat {
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

fn extract(bsa: &bsa::BSAFile, info: &Arguments) -> Result<()> {
    let archive_path = &info.extractfile.replace('/', "\\");
    let extract_path = &info.extractfile.replace('\\', "/");

    if !bsa.exists(archive_path) {
        panic!(
            "ERROR: file '{}' not found
In archive: {}",
            info.extractfile, info.filename
        )
    }

    let rel_path = Path::new(&extract_path);
    let mut target = PathBuf::from(&info.outdir);
    if info.fullpath {
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
    let data = bsa.get_file(archive_path)?;

    // Write the file to disk
    println!(
        "Extracting {} to {}",
        info.extractfile,
        target.to_str().unwrap()
    );
    let f = File::create(target).expect("Unable to create file");
    let mut f = BufWriter::new(f);
    f.write_all(&data)?;
    f.flush()?;
    Ok(())
}

fn extractall(bsa: &bsa::BSAFile, info: &Arguments) -> Result<()> {
    // Get the list of files present in the archive
    let list = bsa.get_list();
    let pb = ProgressBar::new(list.len() as u64);

    for file in list {
        pb.inc(1);
        let extract_path = file.name.replace('\\', "/");

        // Get the target path (the path the file will be extracted to)
        let target = Path::new(&info.outdir).join(extract_path);

        // Create the directory hierarchy
        fs::create_dir_all(target.parent().unwrap()).unwrap();

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

fn create(bsa: &mut bsa::BSAFile, info: &Arguments) -> Result<()> {
    bsa.create(&info.filename, &info.filenames)?;
    Ok(())
}

fn main() {
    let info = parse_options();

    // Open file
    let mut bsa: bsa::BSAFile = bsa::BSAFile::default();
    if ["list", "extract", "extractall"].contains(&info.mode.as_str()) {
        // read header
        bsa.open(info.filename.to_string()).unwrap();
    }

    match info.mode.as_str() {
        "list" => list(&bsa, &info),
        "extract" => extract(&bsa, &info).unwrap(),
        "extractall" => extractall(&bsa, &info).unwrap(),
        "create" => create(&mut bsa, &info).unwrap(),
        _ => println!("Unsupported mode. That is not supposed to happen."),
    }
}
