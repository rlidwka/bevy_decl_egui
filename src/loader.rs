use bevy::asset::{AssetLoader, AsyncReadExt};
use bevy::prelude::*;

use crate::{egui, Label};

#[derive(Asset, TypePath, Debug)]
pub struct EguiAsset {
    pub window: crate::model::Window,
}

impl EguiAsset {
    pub fn show(&self, data: &mut dyn Reflect, ctx: &mut egui::Context) {
        self.window.show(data, ctx);
    }
}

#[derive(Default)]
pub struct EguiAssetLoader;

impl AssetLoader for EguiAssetLoader {
    type Asset = EguiAsset;
    type Error = anyhow::Error;
    type Settings = EguiAssetLoaderSettings;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            if settings.version == 0 {
                return Err(anyhow::anyhow!("
Please use `asset_server.load_uiconf` instead of `asset_server.load`.

Add `use bevy_uiconf_egui::AssetServerExt;` to access it."));
            }

            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).await?;

            Ok(EguiAsset {
                window: crate::model::Root::read(&buffer)?,
                //hash: egui::Id::new((load_context.asset_path(), /*settings.version*/)),
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gui"]
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct EguiAssetLoaderSettings {
    pub version: u32,
}
