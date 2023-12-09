use bevy::asset::{AssetLoader, AsyncReadExt};
use bevy::prelude::*;

use crate::model;

#[derive(Asset, TypePath, Clone, Debug)]
pub struct EguiAsset(pub model::Window);

#[derive(Default)]
pub struct EguiAssetLoader;

impl AssetLoader for EguiAssetLoader {
    type Asset = EguiAsset;
    type Error = anyhow::Error;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).await?;

            let file: model::Root = jomini::text::de::from_utf8_slice(&buffer)?;
            Ok(EguiAsset(file.window))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gui"]
    }
}
