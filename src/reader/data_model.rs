use bevy::reflect::prelude::*;

pub trait ResolveBinding {
    type Item;

    fn resolve(
        &self,
        data: &dyn Reflect,
    ) -> anyhow::Result<Self::Item>;
}

pub trait ResolveBindingRef {
    type Item;

    fn resolve_ref<'data>(
        &'data self,
        data: &'data dyn Reflect,
    ) -> anyhow::Result<&'data Self::Item>;
}

#[derive(Reflect, Debug, Default)]
#[reflect(Default)]
pub struct Trigger(u32);

impl Trigger {
    pub fn check_reset(&mut self) -> bool {
        let triggered = self.0 > 0;
        self.0 = 0;
        triggered
    }

    pub fn get_count(&self) -> u32 {
        self.0
    }

    pub fn trigger(&mut self) {
        self.0 += 1;
    }
}
