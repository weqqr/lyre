use byteorder::{BigEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Read, Cursor};
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid signature: 0x{0:08X}")]
    InvalidSignature(u32),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Class {

}

impl Class {
    pub fn from_file(class_path: &str, class_name: &str) -> Result<Self> {
        let mut path = PathBuf::from(class_path);
        path.push(class_name);
        path.set_extension("class");

        let data = std::fs::read(path)?;

        Self::from_bytes(&data)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let magic = cursor.read_u32::<BigEndian>()?;
        if magic != 0xCAFEBABE {
            return Err(Error::InvalidSignature(magic));
        }

        Ok(Self {})
    }
}

fn main() {
    let mut args = std::env::args();
    let _ = args.next().unwrap();
    let class_path = args.next().unwrap();
    let main_class = args.next().unwrap();

    let class = Class::from_file(&class_path, &main_class).unwrap();

    println!("Hello, world!");
}
