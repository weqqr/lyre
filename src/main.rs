#![allow(dead_code)]

mod class;

use class::Class;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid signature: 0x{0:08X}")]
    InvalidSignature(u32),

    #[error("Invalid version: {} {}", major, minor)]
    InvalidVersion { major: u16, minor: u16 },

    #[error("Invalid constant tag: {0}")]
    InvalidConstantTag(u8),

    #[error("Invalid access flags: 0x{0:04X}")]
    InvalidAccessFlags(u16),

    #[error("Invalid constant pool index")]
    InvalidConstantPoolIndex,

    #[error("Invalid constant pool type")]
    InvalidConstantPoolType,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

fn main() {
    let mut args = std::env::args();
    let _executable = args.next().unwrap();
    let class_path = args.next().unwrap();
    let main_class = args.next().unwrap();

    let class = Class::from_file(&class_path, &main_class).unwrap();
    let main = class.method("main").unwrap();
    println!("{}{}", main.name(), main.descriptor());
    println!("{:#?}", class);
}
