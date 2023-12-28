use std::sync::atomic::AtomicBool;

use anyhow::{anyhow, Context};
use bevy::reflect::{Reflect, ReflectMut, ReflectRef, List};
use jomini::TextToken;
use smol_str::SmolStr;

use super::data_model::{ResolveBinding, ResolveBindingRef};
use super::error::Error;
use super::{reader, ReadUiconf};


#[derive(Debug)]
pub struct BindingRef<T: ?Sized> {
    name: SmolStr,
    warned: AtomicBool,
    _marker: std::marker::PhantomData<T>,
}

impl<T: ?Sized> BindingRef<T> {
    fn change_type<U>(self) -> BindingRef<U> {
        BindingRef {
            name: self.name,
            warned: self.warned,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: ?Sized> ReadUiconf for BindingRef<T> {
    fn read_uiconf(value: &reader::Reader) -> Result<Self, Error> {
        let TextToken::Unquoted(scalar) = value.token() else {
            return Err(Error::invalid_type(value, value.token_type(), "unquoted scalar"));
        };

        let string = scalar.to_string();
        if let Some(reference) = string.strip_prefix('@') {
            Ok(BindingRef {
                name: reference.into(),
                warned: AtomicBool::new(false),
                _marker: std::marker::PhantomData,
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

impl<T: ?Sized> BindingRef<T> {
    pub fn resolve_list_ref<'data>(
        &'data self,
        data: &'data dyn Reflect,
    ) -> anyhow::Result<&'data dyn List> {
        (|| -> anyhow::Result<&'data dyn List> {
            let ReflectRef::Struct(value) = data.reflect_ref() else {
                return Err(anyhow!("expected struct"));
            };
            let value = value.field(&self.name).context("key not found")?;

            let ReflectRef::List(value) = value.reflect_ref() else {
                return Err(anyhow!(
                    "expected list, found {}",
                    value.get_represented_type_info().map(|info| info.type_path()).unwrap_or("<unknown>")
                ));
            };
            Ok(value)
        })().map_err(|err| {
            if !self.warned.fetch_or(true, std::sync::atomic::Ordering::Relaxed) {
                bevy::log::warn!("failed to resolve binding @{}: {}", self.name, err);
            }
            err
        })
    }

    pub fn resolve_list_mut<'data>(
        &'data self,
        data: &'data mut dyn Reflect,
    ) -> anyhow::Result<&'data mut dyn List> {
        let _ = self.resolve_list_ref(data)?;

        // all errors should've been catched by `resolve_ref` above
        let ReflectMut::Struct(value) = data.reflect_mut() else { unreachable!() };
        let value = value.field_mut(&self.name).unwrap();

        let ReflectMut::List(value) = value.reflect_mut() else { unreachable!() };
        Ok(value)
    }
}

impl<T: Reflect> BindingRef<T> {
    pub fn resolve_ref<'data>(
        &'data self,
        data: &'data dyn Reflect,
    ) -> anyhow::Result<&T> {
        (|| -> anyhow::Result<&'data T> {
            let ReflectRef::Struct(value) = data.reflect_ref() else {
                return Err(anyhow!("expected struct"));
            };
            let value = value.field(&self.name).context("key not found")?;
            value.downcast_ref::<T>().ok_or_else(||
                anyhow!(
                    "expected type {}, found {}",
                    std::any::type_name::<T>(),
                    value
                        .get_represented_type_info()
                        .map(|info| info.type_path())
                        .unwrap_or("<unknown>")
                )
            )
        })().map_err(|err| {
            if !self.warned.fetch_or(true, std::sync::atomic::Ordering::Relaxed) {
                bevy::log::warn!("failed to resolve binding @{}: {}", self.name, err);
            }
            err
        })
    }

    pub fn resolve_mut<'data>(
        &'data self,
        data: &'data mut dyn Reflect,
    ) -> anyhow::Result<&'data mut T> {
        let _ = self.resolve_ref(data)?;

        // all errors should've been catched by `resolve_ref` above
        let ReflectMut::Struct(value) = data.reflect_mut() else { unreachable!() };
        let value = value.field_mut(&self.name).unwrap();
        Ok(value.downcast_mut::<T>().unwrap())
    }
}

#[derive(Debug)]
pub enum Binding<T> {
    Ref(BindingRef<T>),
    Value(T),
}

impl<T> Binding<T> {
    pub fn map_value<U, F: FnOnce(T) -> U>(self, f: F) -> Binding<U> {
        match self {
            Binding::Ref(binding) => Binding::Ref(binding.change_type()),
            Binding::Value(value) => Binding::Value(f(value)),
        }
    }
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

impl<T: Reflect + Copy> ResolveBinding for Binding<T> {
    type Item = T;

    fn resolve(&self, data: &dyn Reflect) -> anyhow::Result<Self::Item> {
        self.resolve_ref(data).copied()
    }
}

impl<T: Reflect> ResolveBindingRef for Binding<T> {
    type Item = T;

    fn resolve_ref<'data>(&'data self, data: &'data dyn Reflect) -> anyhow::Result<&'data Self::Item> {
        match self {
            Binding::Ref(binding) => binding.resolve_ref(data),
            Binding::Value(value) => Ok(value),
        }
    }
}
