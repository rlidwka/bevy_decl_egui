use bevy::asset::{AssetLoader, AsyncReadExt};
use bevy::prelude::*;

use crate::{egui, Label};

#[derive(Asset, TypePath, Debug)]
pub struct EguiAsset<L: Label>{
    pub window: crate::model::Window,
    hash: egui::Id,
    _labels: std::marker::PhantomData<L>,
}

impl<L: Label> EguiAsset<L> {
    pub fn show(&self, data: &mut dyn Reflect, ctx: &mut egui::Context) {
        self.window.show(data, ctx);
    }
}

pub trait LabelToId<L: Label> {
    fn to_id(&self) -> egui::Id;
}

impl LabelToId<String> for &str {
    fn to_id(&self) -> egui::Id {
        // assert_eq!(egui::Id::new("test"), egui::Id::new("test".to_owned()));
        egui::Id::new(*self)
    }
}

impl<T: Label> LabelToId<T> for T {
    fn to_id(&self) -> egui::Id {
        egui::Id::new(self)
    }
}

pub struct EguiAssetLoader<L> {
    _label: std::marker::PhantomData<L>,
}

impl<L: Label> AssetLoader for EguiAssetLoader<L> {
    type Asset = EguiAsset<L>;
    type Error = anyhow::Error;
    type Settings = EguiAssetLoaderSettings;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        settings: &'a Self::Settings,
        load_context: &'a mut bevy::asset::LoadContext,
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
                hash: egui::Id::new((load_context.asset_path(), /*settings.version*/)),
                _labels: Default::default(),
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gui"]
    }
}

impl<L> Default for EguiAssetLoader<L> {
    fn default() -> Self {
        Self { _label: Default::default() }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct EguiAssetLoaderSettings {
    pub version: u32,
}
