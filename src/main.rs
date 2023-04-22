use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write, Seek};
use std::path::{Path, PathBuf};
use std::ffi::OsString;

#[derive(Debug)]
struct AddonFile {
    name: String,
    size: u32,
}

fn ztstr(file: &mut File) -> io::Result<String> {
    let mut buffer = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        file.read_exact(&mut byte)?;
        if byte[0] == 0 {
            break;
        }
        buffer.push(byte[0]);
    }
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

fn fopenwb(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    File::create(path)
}

fn is_directory(path: &Path) -> bool {
    path.is_dir()
}

fn extract_file(input_path: &Path, output_dir: &Path) -> io::Result<()> {
    let mut _filecnt = 0;
    let mut addon_files: Vec<AddonFile> = Vec::new();
    let mut input = File::open(input_path)?;

    let mut i = [0u8; 4];
    input.read_exact(&mut i)?;

    if u32::from_le_bytes(i) != 0x44414d47 {
        eprintln!("Input file is not an addon");
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Input file is not an addon"));
    }

    input.seek(io::SeekFrom::Current(18))?;
    let addon_name = ztstr(&mut input)?;
    ztstr(&mut input)?; // Ignore addon description
    ztstr(&mut input)?; // Ignore addon author
    input.seek(io::SeekFrom::Current(4))?;

    println!("Attempting to extract file names...");
    loop {
        let mut filenum = [0u8; 4];
        input.read_exact(&mut filenum)?;
        if u32::from_le_bytes(filenum) == 0 {
            break;
        }

        _filecnt += 1;
        let name = ztstr(&mut input)?;
        let mut size = [0u8; 4];
        input.read_exact(&mut size)?;

        addon_files.push(AddonFile {
            name,
            size: u32::from_le_bytes(size),
        });

        input.seek(io::SeekFrom::Current(8))?;
    }

    if addon_files.is_empty() {
        println!("Addon is empty");
        return Ok(());
    }

    let addon_path = output_dir.join(&addon_name);
    fs::create_dir_all(&addon_path)?;

    for file in addon_files {
        println!("Extracting {} ({}B)", file.name, file.size);
        let output_file_path = create_output_file_path(&addon_path, &file.name);
        let mut output = fopenwb(&output_file_path)?;
        let mut buffer = vec![0u8; file.size as usize];
        input.read_exact(&mut buffer)?;
        output.write_all(&buffer)?;
        output.flush()?;
    }

    println!("Finished extracting addon {}", addon_name);

    Ok(())
}

fn create_output_file_path(addon_path: &Path, file_name: &str) -> PathBuf {
    let components: Vec<&str> = file_name.split('/').collect();
    let mut path = addon_path.to_owned();
    for component in components {
        path = path.join(component);
    }
    path
}

fn main() -> io::Result<()> {
    println!("GMAD Extractor by Votton");

    let args: Vec<OsString> = env::args_os().collect();
    if args.len() < 3 {
        eprintln!("No file/directory specified");
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No file/directory specified"));
    }

    let input_path = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);

    if !output_dir.exists() {
        fs::create_dir_all(&output_dir).expect("Error creating output directory");
    }

    if !is_directory(output_dir) {
        eprintln!("{} is not a directory!", output_dir.display());
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Output path is not a directory"));
    }

    fn visit_dirs(dir: &Path, output_dir: &Path, cb: &dyn Fn(&Path, &Path) -> io::Result<()>) -> io::Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, output_dir, cb)?;
                } else {
                    cb(&path, output_dir)?;
                }
            }
        }
        Ok(())
    }

    if is_directory(input_path) {
        visit_dirs(input_path, output_dir, &extract_file)?;
    } else {
        extract_file(input_path, output_dir)?;
    }

    Ok(())
}