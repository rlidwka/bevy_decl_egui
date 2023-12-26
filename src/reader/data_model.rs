use std::any::Any;

use bevy::utils::HashMap;

pub trait ResolveBinding {
    type Item;
    fn resolve<'data>(&'data self, data: &'data DataModel) -> anyhow::Result<&'data Self::Item>;
    fn resolve_mut<'data>(&'data mut self, data: &'data mut DataModel) -> anyhow::Result<&'data mut Self::Item>;
}

#[derive(Default)]
pub struct DataModel(HashMap<String, DataItem>);

impl DataModel {
    pub fn new() -> Self {
        Self(HashMap::default())
    }

    pub fn set<T: Any>(&mut self, key: &str, value: T) {
        self.0.insert(key.to_string(), DataItem::new(value));
    }

    pub fn get<T: Any>(&self, key: &str) -> anyhow::Result<&T> {
        self.0
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("key `{}` not found", key))?
            .get()
    }

    pub fn get_mut<T: Any>(&mut self, key: &str) -> anyhow::Result<&mut T> {
        self.0
            .get_mut(key)
            .ok_or_else(|| anyhow::anyhow!("key `{}` not found", key))?
            .get_mut()
    }
}

struct DataItem {
    type_name: &'static str,
    value: Box<dyn Any>,
}

impl DataItem {
    fn new<T: Any>(value: T) -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            value: Box::new(value),
        }
    }

    fn get<T: Any>(&self) -> anyhow::Result<&T> {
        self.value.downcast_ref::<T>().ok_or_else(|| {
            anyhow::anyhow!(
                "expected type {}, found {}",
                std::any::type_name::<T>(),
                self.type_name
            )
        })
    }

    fn get_mut<T: Any>(&mut self) -> anyhow::Result<&mut T> {
        self.value.downcast_mut::<T>().ok_or_else(|| {
            anyhow::anyhow!(
                "expected type {}, found {}",
                std::any::type_name::<T>(),
                self.type_name
            )
        })
    }
}
