use bevy::prelude::*;
use bevy::asset::AssetLoader;
use bevy::asset::io::Reader; // Correct import for Bevy 0.13+ IO
use thiserror::Error;

/// A simple wrapper around the raw bytes of an .odin file.
/// We don't parse it here to avoid dependency on odin2-core in the game client.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct OdinAsset {
    pub bytes: Vec<u8>,
}

#[derive(Default)]
pub struct OdinAssetLoader;

/// Possible errors during asset loading
#[derive(Debug, Error)]
pub enum OdinAssetLoaderError {
    #[error("Could not read asset: {0}")]
    Io(#[from] std::io::Error),
}

impl AssetLoader for OdinAssetLoader {
    type Asset = OdinAsset;
    type Settings = ();
    type Error = OdinAssetLoaderError;

    // Bevy 0.17 signature
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(OdinAsset { bytes })
    }

    fn extensions(&self) -> &[&str] {
        &["odin"]
    }
}
