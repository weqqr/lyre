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

    #[error("Invalid version: {} {}", major, minor)]
    InvalidVersion { major: u16, minor: u16 },

    #[error("Invalid constant tag: {0}")]
    InvalidConstantTag(u8),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

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

pub struct ConstantPool {
    pool: Vec<Constant>,
}

impl ConstantPool {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let constant_pool_count = r.read_u16::<BigEndian>()?;
        let mut pool = Vec::new();
        for _ in 0..constant_pool_count-1 {
            let constant = Constant::read(r)?;
            println!("{:?}", constant);
            pool.push(constant);
        }

        Ok(Self {
            pool
        })
    }
}

pub struct Attribute {
    attribute_name_index: u16,
    info: Vec<u8>,
}

impl Attribute {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let attribute_name_index = r.read_u16::<BigEndian>()?;
        let info_length = r.read_u32::<BigEndian>()? as usize;
        let mut info = vec![0; info_length];
        r.read_exact(&mut info);
        Ok(Attribute {
            attribute_name_index,
            info
        })
    }
}

pub struct Field {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Vec<Attribute>,
}

impl Field {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let access_flags = r.read_u16::<BigEndian>()?;
        let name_index = r.read_u16::<BigEndian>()?;
        let descriptor_index = r.read_u16::<BigEndian>()?;
        let attributes_count = r.read_u16::<BigEndian>()?;
        let mut attributes = Vec::new();
        for _ in 0..attributes_count {
            attributes.push(Attribute::read(r)?);
        }

        Ok(Field {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

pub struct Method {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Vec<Attribute>,
}

impl Method {
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let access_flags = r.read_u16::<BigEndian>()?;
        let name_index = r.read_u16::<BigEndian>()?;
        let descriptor_index = r.read_u16::<BigEndian>()?;
        let attributes_count = r.read_u16::<BigEndian>()?;
        let mut attributes = Vec::new();
        for _ in 0..attributes_count {
            attributes.push(Attribute::read(r)?);
        }

        Ok(Method {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

pub struct Class {
    major: u16,
    minor: u16,
    constant_pool: ConstantPool,
    access_flags: u16,
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

        let minor = cursor.read_u16::<BigEndian>()?;
        let major = cursor.read_u16::<BigEndian>()?;
        if major > 56 && (minor != 0 || minor != 65535) {
            return Err(Error::InvalidVersion{ major, minor });
        }

        let constant_pool = ConstantPool::read(&mut cursor)?;

        let access_flags = cursor.read_u16::<BigEndian>()?;
        let this_class = cursor.read_u16::<BigEndian>()?;
        let super_class = cursor.read_u16::<BigEndian>()?;

        let interfaces_count = cursor.read_u16::<BigEndian>()?;
        let mut interfaces = Vec::new();
        for _ in 0..interfaces_count {
            let interface = cursor.read_u16::<BigEndian>()?;
            interfaces.push(interface);
        }

        let fields_count = cursor.read_u16::<BigEndian>()?;
        let mut fields = Vec::new();
        for _ in 0..fields_count {
            fields.push(Field::read(&mut cursor)?);
        }

        let methods_count = cursor.read_u16::<BigEndian>()?;
        let mut methods = Vec::new();
        for _ in 0..methods_count {
            methods.push(Method::read(&mut cursor)?);
        }

        let attributes_count = cursor.read_u16::<BigEndian>()?;
        let mut attributes = Vec::new();
        for _ in 0..attributes_count {
            attributes.push(Attribute::read(&mut cursor)?);
        }

        Ok(Self {
            major,
            minor,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes,
        })
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
