use std::any::Any;
use std::sync::atomic::AtomicBool;

use jomini::TextToken;

use super::data_model::{ResolveBinding, DataModel};
use super::error::Error;
use super::{reader, ReadUiconf};


#[derive(Debug)]
pub struct BindingRef {
    name: String,
    warned: AtomicBool,
}

impl ReadUiconf for BindingRef {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let TextToken::Unquoted(scalar) = value.token() else {
            return Err(Error::invalid_type(value, value.token_type(), "unquoted scalar"));
        };

        let string = scalar.to_string();
        if let Some(reference) = string.strip_prefix('@') {
            Ok(BindingRef {
                name: reference.to_string(),
                warned: AtomicBool::new(false),
            })
        } else {
            Err(Error::invalid_value(
                value,
                &string,
                "@ref",
            ))
        }
    }
}

#[derive(Debug)]
pub enum Binding<T> {
    Ref(BindingRef),
    Value(T),
}

impl<T: ReadUiconf> ReadUiconf for Binding<T> {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let binding = BindingRef::read_uiconf(value);
        if let Ok(binding) = binding {
            Ok(Binding::Ref(binding))
        } else {
            Ok(Binding::Value(T::read_uiconf(value)?))
        }
    }
}

impl<T: Any> ResolveBinding for Binding<T> {
    type Item = T;

    fn resolve<'data>(&'data self, data: &'data DataModel) -> anyhow::Result<&'data Self::Item> {
        match self {
            Binding::Ref(binding) => data.get(&binding.name).map_err(|err| {
                if !binding.warned.fetch_or(true, std::sync::atomic::Ordering::Relaxed) {
                    bevy::log::warn!("failed to resolve binding @{}: {}", binding.name, err);
                }
                err
            }),
            Binding::Value(value) => Ok(value),
        }
    }

    fn resolve_mut<'data>(&'data mut self, data: &'data mut DataModel) -> anyhow::Result<&'data mut Self::Item> {
        match self {
            Binding::Ref(binding) => data.get_mut(&binding.name).map_err(|err| {
                if !binding.warned.fetch_or(true, std::sync::atomic::Ordering::Relaxed) {
                    bevy::log::warn!("failed to resolve binding @{}: {}", binding.name, err);
                }
                err
            }),
            Binding::Value(value) => Ok(value),
        }
    }
}
