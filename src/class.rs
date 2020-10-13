use bitflags::bitflags;
use byteorder::{BigEndian, ReadBytesExt};
use crate::{Error, Result};
use std::io::{Read, Cursor};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Version {
    major: u16,
    minor: u16,
}

impl Version {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let minor = r.read_u16::<BigEndian>()?;
        let major = r.read_u16::<BigEndian>()?;
        if major > 56 && minor != 0 && minor != 65535 {
            Err(Error::InvalidVersion{ major, minor })
        } else {
            Ok(Version { major, minor })
        }
    }

}

pub enum ReferenceKind {
    GetField,
    GetStatic,
    PutField,
    PutStatic,
    InvokeVirtual,
    InvokeStatic,
    InvokeSpecial,
    NewInvokeSpecial,
    InvokeInterface
}

bitflags! {
    struct AccessFlags : u16 {
        const PUBLIC       = 0x0001;
        const PRIVATE      = 0x0002;
        const PROTECTED    = 0x0004;
        const STATIC       = 0x0008;
        const FINAL        = 0x0010;
        const SYNCHRONIZED = 0x0020;
        const BRIDGE       = 0x0040;
        const VARARGS      = 0x0080;
        const NATIVE       = 0x0100;
        const ABSTRACT     = 0x0400;
        const STRICT       = 0x0800;
        const SYNTHETIC    = 0x1000;
    }
}

impl AccessFlags {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let bits = r.read_u16::<BigEndian>()?;
        Self::from_bits(bits).ok_or(Error::InvalidAccessFlags(bits))
    }
}

#[derive(Debug)]
pub enum Constant {
    Class {
        name_index: usize,
    },
    FieldRef {
        class_index: usize,
        name_and_type_index: usize,
    },
    MethodRef {
        class_index: usize,
        name_and_type_index: usize,
    },
    InterfaceMethodRef {
        class_index: usize,
        name_and_type_index: usize,
    },
    String {
        string_index: usize,
    },
    Integer {
        data: u32,
    },
    Float {
        data: f32,
    },
    Long {
        data: u64,
    },
    Double {
        data: f64,
    },
    NameAndType {
        name_index: usize,
        descriptor_index: usize,
    },
    Utf8 {
        data: String,
    },
    MethodHandle {
        reference_kind: u16,
        reference_index: usize,
    },
    MethodType {
        descriptor_index: usize,
    },
    Dynamic {
        bootstrap_method_attr_index: usize,
        name_and_type_index: usize,
    },
    InvokeDynamic {
        bootstrap_method_attr_index: usize,
        name_and_type_index: usize,
    },
    Module {
        name_index: usize,
    },
    Package {
        name_index: usize,
    }
}

impl Constant {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let tag = r.read_u8()?;
        let constant = match tag {
            1 => {
                // FIXME: that's not how "Modified UTF-8" decoding works.
                let length = r.read_u16::<BigEndian>()?.into();
                let mut data = vec![0; length];
                r.read_exact(&mut data)?;
                let data = String::from_utf8(data).unwrap();
                Constant::Utf8 {
                    data,
                }
            }
            3 => Constant::Integer {
                data: r.read_u32::<BigEndian>()?,
            },
            4 => Constant::Float {
                data: r.read_f32::<BigEndian>()?,
            },
            5 => Constant::Long {
                data: r.read_u64::<BigEndian>()?,
            },
            6 => Constant::Double {
                data: r.read_f64::<BigEndian>()?,
            },
            7 => Constant::Class {
                name_index: r.read_u16::<BigEndian>()?.into(),
            },
            8 => Constant::String {
                string_index: r.read_u16::<BigEndian>()?.into(),
            },
            9 => Constant::FieldRef {
                class_index: r.read_u16::<BigEndian>()?.into(),
                name_and_type_index: r.read_u16::<BigEndian>()?.into(),
            },
            10 => Constant::MethodRef {
                class_index: r.read_u16::<BigEndian>()?.into(),
                name_and_type_index: r.read_u16::<BigEndian>()?.into(),
            },
            11 => Constant::InterfaceMethodRef {
                class_index: r.read_u16::<BigEndian>()?.into(),
                name_and_type_index: r.read_u16::<BigEndian>()?.into(),
            },
            12 => Constant::NameAndType {
                name_index: r.read_u16::<BigEndian>()?.into(),
                descriptor_index: r.read_u16::<BigEndian>()?.into(),
            },
            15 => Constant::MethodHandle {
                reference_index: r.read_u16::<BigEndian>()?.into(),
                reference_kind: r.read_u16::<BigEndian>()?,
            },
            16 => Constant::MethodType {
                descriptor_index: r.read_u16::<BigEndian>()?.into()
            },
            17 => Constant::Dynamic {
                bootstrap_method_attr_index: r.read_u16::<BigEndian>()?.into(),
                name_and_type_index: r.read_u16::<BigEndian>()?.into(),
            },
            18 => Constant::InvokeDynamic {
                bootstrap_method_attr_index: r.read_u16::<BigEndian>()?.into(),
                name_and_type_index: r.read_u16::<BigEndian>()?.into(),
            },
            19 => Constant::Module {
                name_index: r.read_u16::<BigEndian>()?.into(),
            },
            20 => Constant::Package {
                name_index: r.read_u16::<BigEndian>()?.into(),
            },
            _ => return Err(Error::InvalidConstantTag(tag)),
        };

        Ok(constant)
    }
}

#[derive(Debug)]
pub struct ConstantPool {
    pool: Vec<Constant>,
}

impl ConstantPool {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let constant_pool_count = r.read_u16::<BigEndian>()?;
        let mut pool = Vec::new();
        for _ in 0..constant_pool_count-1 {
            let constant = Constant::read(r)?;
            pool.push(constant);
        }

        Ok(Self {
            pool
        })
    }
}

fn read_vec<T, F, R>(r: &mut R, f: F) -> Result<Vec<T>> where
    R: Read,
    F: Fn(&mut R) -> Result<T>
{
    let count = r.read_u16::<BigEndian>()?;
    let mut elements = Vec::new();
    for _ in 0..count {
        elements.push(f(r)?);
    }

    Ok(elements)
}

#[derive(Debug)]
pub struct Attribute {
    attribute_name_index: u16,
    info: Vec<u8>,
}

impl Attribute {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let attribute_name_index = r.read_u16::<BigEndian>()?;
        let info_length = r.read_u32::<BigEndian>()? as usize;
        let mut info = vec![0; info_length];
        r.read_exact(&mut info)?;
        Ok(Attribute {
            attribute_name_index,
            info
        })
    }
}

#[derive(Debug)]
pub struct Field {
    access_flags: AccessFlags,
    name_index: u16,
    descriptor_index: u16,
    attributes: Vec<Attribute>,
}

impl Field {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        Ok(Field {
            access_flags: AccessFlags::read(r)?,
            name_index: r.read_u16::<BigEndian>()?,
            descriptor_index: r.read_u16::<BigEndian>()?,
            attributes: read_vec(r, Attribute::read)?,
        })
    }
}

#[derive(Debug)]
pub struct Method {
    access_flags: AccessFlags,
    name_index: u16,
    descriptor_index: u16,
    attributes: Vec<Attribute>,
}

impl Method {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        Ok(Method {
            access_flags: AccessFlags::read(r)?,
            name_index: r.read_u16::<BigEndian>()?,
            descriptor_index: r.read_u16::<BigEndian>()?,
            attributes: read_vec(r, Attribute::read)?,
        })
    }
}

#[derive(Debug)]
pub struct Class {
    version: Version,
    constant_pool: ConstantPool,
    access_flags: AccessFlags,
    this_class: u16,
    super_class: u16,
    interfaces: Vec<u16>,
    fields: Vec<Field>,
    methods: Vec<Method>,
    attributes: Vec<Attribute>,
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

        Ok(Self {
            version: Version::read(&mut cursor)?,
            constant_pool: ConstantPool::read(&mut cursor)?,
            access_flags: AccessFlags::read(&mut cursor)?,
            this_class: cursor.read_u16::<BigEndian>()?,
            super_class: cursor.read_u16::<BigEndian>()?,
            interfaces: read_vec(&mut cursor, |r| Ok(r.read_u16::<BigEndian>()?))?,
            fields: read_vec(&mut cursor, Field::read)?,
            methods: read_vec(&mut cursor, Method::read)?,
            attributes: read_vec(&mut cursor, Attribute::read)?,
        })
    }
}