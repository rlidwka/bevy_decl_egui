use std::hash::Hash;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::asset::AssetPath;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiContexts;
use loader::{EguiAsset, EguiAssetLoader, EguiAssetLoaderSettings};
use reader::data_model::Trigger;
use serde::Deserialize;

mod const_concat;
pub mod loader;
pub mod model;
pub mod reader;

#[derive(Default)]
pub struct UiconfPlugin<L> {
    _label: std::marker::PhantomData<L>,
}

impl<L> UiconfPlugin<L> {
    pub fn new() -> Self {
        Self { _label: Default::default() }
    }
}

impl<L: Label> Plugin for UiconfPlugin<L> {
    fn build(&self, app: &mut App) {
        app.init_asset::<EguiAsset<L>>();
        app.init_asset_loader::<EguiAssetLoader<L>>();
        app.register_type::<Trigger>();
    }
}

pub trait Label: TypePath + for<'a> Deserialize<'a> + PartialEq + Eq + Hash + Send + Sync {}
impl<L> Label for L where L: TypePath + for<'a> Deserialize<'a> + PartialEq + Eq + Hash + Send + Sync {}

pub use loader::EguiAsset as UiconfWindow;

// re-export egui
pub use bevy_inspector_egui::egui;

pub trait AssetServerExt {
    fn load_uiconf<'a, L: Label>(&self, path: impl Into<AssetPath<'a>>) -> Handle<EguiAsset<L>>;
}

impl AssetServerExt for AssetServer {
    fn load_uiconf<'a, L: Label>(&self, path: impl Into<AssetPath<'a>>) -> Handle<EguiAsset<L>> {
        let counter = AtomicU32::new(1);
        self.load_with_settings(path, move |settings: &mut EguiAssetLoaderSettings| {
            settings.version = counter.fetch_add(1, Ordering::Relaxed);
        })
    }
}

pub fn clear_egui_state_on_reload<L: Label>(
    mut events: EventReader<AssetEvent<EguiAsset<L>>>,
    mut egui_contexts: EguiContexts,
) {
    if !events.is_empty() {
        egui_contexts.ctx_mut().memory_mut(|mem| *mem = Default::default());
    }
    events.clear();
}
