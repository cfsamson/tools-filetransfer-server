use failure::{err_msg, Error};
fn main() -> Result<(), Error> {
    connect();
    //display(folder);

    Ok(())
}

fn connect() {
    match server() {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e);
            println!("reconnecting...");
            connect();
        }
    }
}

use bincode::serialize;
use std::io::Write;
use std::net::TcpStream;
fn server() -> Result<(), Error> {
    let mut reconnect_flag = false;
    let listener = std::net::TcpListener::bind("0.0.0.0:8000")?;
    println!("Server listening on: http://{}", "0.0.0.0:8000");

    for stream in listener.incoming() {
        let mut stream: TcpStream = stream?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(4)))?;
        println!("{:?}", stream);

        // testing, just accept any
        let mut buffer = [0; 128];
        let b_count = stream.read(&mut buffer)?;

        let buffer = String::from_utf8_lossy(&buffer[0..b_count]);
        let buffer_tr = buffer;

        if buffer_tr == "shared key" {
            //stream.write(b"Connected. Please provide a path:")?; - messed up the response
            let folders = read_files(r"./", None)?;
            let mut data = serialize_folder(folders)?;
            println!("{}", data.len());
            stream.write_all(&mut data)?;
            stream.flush()?;
        }
    }

    Ok(())
}

fn display(folder: Folder) {
    println!(
        "DISPLAYING FOR {} ({})",
        folder.name.to_uppercase(),
        &folder.path.to_string_lossy()
    );
    for file in folder.files {
        println!(
            "name: {}, ext: {}",
            file.name,
            file.ext.unwrap_or("".into())
        );
    }

    for dir in &folder.sub_folders {
        println!("dir: {}", dir.name);
    }

    for dir in folder.sub_folders {
        display(dir);
    }
}

fn serialize_folder(folder: Folder) -> Result<Vec<u8>, bincode::Error> {
    serialize(&folder)
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
pub fn read_files(
    path: impl Into<PathBuf>,
    relative_path: Option<PathBuf>,
) -> Result<Folder, Error> {
    let path = path.into();
    let root = fs::read_dir(&path)?;

    if !path.is_dir() {
        panic!("Path is not a directory");
    }

    let mut relative = relative_path.unwrap_or(PathBuf::new());

    let dir_name = path
        .file_stem()
        .unwrap_or(std::ffi::OsStr::new("missing_name"))
        .to_string_lossy()
        .to_string();
    let mut folder = Folder::new(dir_name);
    folder.path = relative.clone();

    for entry in root {
        let entry: fs::DirEntry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            // Hack to avoid .git files and Cargo build directories for now -----------------------
            if entry.file_name() == std::ffi::OsStr::new(".git") || entry.file_name() == std::ffi::OsStr::new("target") {
                continue;
            }
            
            let relative = relative.join(entry.file_name());
            folder.add_sub_folder(read_files(entry.path(), Some(relative.clone()))?);
        } else {
            let file = File::new(entry)?;
            folder.add_file(file);
        }
    }

    Ok(folder)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Folder {
    name: String,
    path: PathBuf,
    sub_folders: Vec<Folder>,
    files: Vec<File>,
}

impl Folder {
    fn new(name: String) -> Folder {
        Folder {
            name,
            path: PathBuf::new(),
            sub_folders: vec![],
            files: vec![],
        }
    }

    fn add_sub_folder(&mut self, sub_folder: Folder) {
        self.sub_folders.push(sub_folder);
    }

    fn add_file(&mut self, file: File) {
        self.files.push(file);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    name: String,
    ext: Option<String>,
    data: Vec<u8>,
}

impl File {
    fn new(entry: fs::DirEntry) -> Result<File, Error> {
        let mut f = fs::File::open(entry.path())?;
        let mut buffer: Vec<u8> = vec![];
        f.read_to_end(&mut buffer)?;

        let path = entry.path();

        let name = path
            .file_name()
            .ok_or(err_msg("File doesn't have a name."))?;
        let ext = path
            .extension()
            .map(|os_str| os_str.to_string_lossy().to_string());

        Ok(File {
            name: name.to_string_lossy().to_string(),
            ext: ext,
            data: buffer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_serializes_and_deserializes() {
        let folders = read_files(r"c:\temp", None).unwrap();
        let mut data = serialize_folder(folders).unwrap();
        let folder: Folder = bincode::deserialize(&data).unwrap();
    }
}
