use std::sync::atomic::{AtomicU32, Ordering};

use bevy::asset::AssetPath;
use bevy::prelude::*;

use self::loader::{EguiAsset, EguiAssetLoader, EguiAssetLoaderSettings};
use self::reader::data_model::Trigger;

mod const_concat;
pub mod loader;
pub mod model;
pub mod reader;

#[derive(Default)]
pub struct UiconfPlugin;

impl Plugin for UiconfPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<EguiAsset>();
        app.init_asset_loader::<EguiAssetLoader>();
        app.register_type::<Trigger>();
    }
}

pub use loader::EguiAsset as UiconfWindow;

// re-export egui
pub use bevy_egui::egui;
pub use bevy_egui::EguiContexts;

pub trait AssetServerExt {
    fn load_uiconf<'a>(&self, path: impl Into<AssetPath<'a>>) -> Handle<EguiAsset>;
}

impl AssetServerExt for AssetServer {
    fn load_uiconf<'a>(&self, path: impl Into<AssetPath<'a>>) -> Handle<EguiAsset> {
        let counter = AtomicU32::new(1);
        self.load_with_settings(path, move |settings: &mut EguiAssetLoaderSettings| {
            settings.version = counter.fetch_add(1, Ordering::Relaxed);
        })
    }
}

pub fn clear_egui_state_on_reload(
    mut events: EventReader<AssetEvent<EguiAsset>>,
    mut egui_contexts: bevy_egui::EguiContexts,
) {
    if !events.is_empty() {
        egui_contexts.ctx_mut().memory_mut(|mem| *mem = Default::default());
    }
    events.clear();
}
