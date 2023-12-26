use std::borrow::Cow;

use jomini::text::ValueReader;
use jomini::{Scalar, TextToken, Utf8Encoding};
use smol_str::SmolStr;

use super::ReadUiconf;
use super::error::Error;

pub struct Reader<'data, 'tokens> {
    reader: ValueReader<'data, 'tokens, Utf8Encoding>,
    path: Vec<SmolStr>,
}

impl<'d, 't> Reader<'d, 't> {
    pub fn new(value: ValueReader<'d, 't, Utf8Encoding>, path: Vec<SmolStr>) -> Self {
        Self { reader: value, path }
    }

    pub fn token(&self) -> &TextToken<'d> {
        self.reader.token()
    }

    pub fn path(&self) -> &[SmolStr] {
        &self.path
    }

    pub fn read<T: ReadUiconf>(&self) -> Result<T, Error> {
        T::read_uiconf(self)
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self.reader.token(), TextToken::Quoted(_) | TextToken::Unquoted(_))
    }

    pub fn read_scalar(&self) -> Result<Scalar<'d>, Error> {
        match self.token() {
            TextToken::Quoted(scalar) => Ok(*scalar),
            TextToken::Unquoted(scalar) => Ok(*scalar),
            _ => Err(Error::invalid_type(self, self.token_type(), "scalar")),
        }
    }

    pub fn read_string(&self) -> Result<String, Error> {
        Ok(self.read_scalar()?.to_string())
    }

    pub fn read_object(
        &self,
    ) -> Result<impl Iterator<Item = (Cow<'d, str>, Reader<'d, 't>)>, Error> {
        match self.token() {
            TextToken::Object { .. } => (),
            TextToken::Array { .. } => (),
            _ => return Err(Error::invalid_type(self, self.token_type(), "object")),
        };

        let object = self.reader.read_object().map_err(|err| Error::deserialize_error(self, err))?;
        let mut fields = object.fields();
        for (_, op, _) in fields.by_ref() {
            if let Some(op) = op {
                return Err(Error::unexpected_operator(self, op));
            }
        }
        if let Some(remainder) = fields.remainder().values().next() {
            let remainder = if let Ok(str) = remainder.read_str() {
                str
            } else {
                Cow::Borrowed("")
            };
            return Err(Error::unexpected_remainder(self, &remainder));
        }
        let path = self.path.clone();
        Ok(object.fields().map(move |(key, _, value)| {
            let mut path = path.clone();
            path.push(key.read_str().into());
            (key.read_str(), Reader::new(value, path))
        }))
    }

    pub fn read_array(&self) -> Result<impl Iterator<Item = Reader<'d, 't>>, Error> {
        match self.token() {
            TextToken::Object { .. } => (),
            TextToken::Array { .. } => (),
            _ => return Err(Error::invalid_type(self, self.token_type(), "array")),
        };

        let array = self.reader.read_array().map_err(|err| Error::deserialize_error(self, err))?;
        let path = self.path.clone();
        let mut index = 0;
        Ok(array.values().map(move |value| {
            let mut path = path.clone();
            path.push(index.to_string().into());
            index += 1;
            Reader::new(value, path)
        }))
    }

    pub fn token_type(&self) -> &'static str {
        match self.token() {
            TextToken::Array { .. }          => "array",
            TextToken::Object { .. }         => "object",
            TextToken::MixedContainer        => "mixed container",
            TextToken::Unquoted(_)           => "unquoted scalar",
            TextToken::Quoted(_)             => "quoted scalar",
            TextToken::Parameter(_)          => "parameter",
            TextToken::UndefinedParameter(_) => "undefined parameter",
            TextToken::Operator(_)           => "operator",
            TextToken::End(_)                => "end",
            TextToken::Header(_)             => "header",
        }
    }
}
