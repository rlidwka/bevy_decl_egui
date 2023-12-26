pub mod binding;
pub mod data_model;
pub mod error;
pub mod reader;

use error::Error;

pub trait ReadUiconf: Sized {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error>;
}

impl ReadUiconf for String {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        Ok(value.read_scalar()?.to_string())
    }
}

impl ReadUiconf for bool {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        value.read_scalar()?.to_bool().map_err(|err| Error::scalar_error(value, err))
    }
}

impl ReadUiconf for u8 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let v = value.read_scalar()?.to_u64().map_err(|err| Error::scalar_error(value, err))?;
        v.try_into().map_err(|_| Error::invalid_value(value, &format!("{}", v), "u8"))
    }
}

impl ReadUiconf for i8 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let v = value.read_scalar()?.to_i64().map_err(|err| Error::scalar_error(value, err))?;
        v.try_into().map_err(|_| Error::invalid_value(value, &format!("{}", v), "i8"))
    }
}

impl ReadUiconf for u16 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let v = value.read_scalar()?.to_u64().map_err(|err| Error::scalar_error(value, err))?;
        v.try_into().map_err(|_| Error::invalid_value(value, &format!("{}", v), "u16"))
    }
}

impl ReadUiconf for i16 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let v = value.read_scalar()?.to_i64().map_err(|err| Error::scalar_error(value, err))?;
        v.try_into().map_err(|_| Error::invalid_value(value, &format!("{}", v), "i16"))
    }
}

impl ReadUiconf for u32 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let v = value.read_scalar()?.to_u64().map_err(|err| Error::scalar_error(value, err))?;
        v.try_into().map_err(|_| Error::invalid_value(value, &format!("{}", v), "u32"))
    }
}

impl ReadUiconf for i32 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let v = value.read_scalar()?.to_i64().map_err(|err| Error::scalar_error(value, err))?;
        v.try_into().map_err(|_| Error::invalid_value(value, &format!("{}", v), "i32"))
    }
}

impl ReadUiconf for u64 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        value.read_scalar()?.to_u64().map_err(|err| Error::scalar_error(value, err))
    }
}

impl ReadUiconf for i64 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        value.read_scalar()?.to_i64().map_err(|err| Error::scalar_error(value, err))
    }
}

impl ReadUiconf for f32 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        Ok(value.read_scalar()?.to_f64().map_err(|err| Error::scalar_error(value, err))? as f32)
    }
}

impl ReadUiconf for f64 {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        value.read_scalar()?.to_f64().map_err(|err| Error::scalar_error(value, err))
    }
}

impl<T: ReadUiconf> ReadUiconf for Vec<T> {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let array = value.read_array()?;
        let mut result = Vec::new();
        for value in array {
            result.push(T::read_uiconf(&value)?);
        }
        Ok(result)
    }
}
