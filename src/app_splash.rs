use crate::AppState;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_kira_audio::AudioSource;

pub struct SplashPlugin;

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.add_loading_state(
            LoadingState::new(AppState::Splash)
                .continue_to_state(AppState::MainMenu)
                .load_collection::<AudioAssets>()
                .load_collection::<TextureAssets>(),
        )
        .add_systems(Startup, setup);
    }
}

#[derive(AssetCollection, Resource)]
pub struct AudioAssets {
    #[asset(path = "audio/flying.ogg")]
    pub flying: Handle<AudioSource>,
}

#[derive(AssetCollection, Resource)]
pub struct TextureAssets {
    #[asset(path = "textures/bevy.png")]
    pub bevy: Handle<Image>,
    #[asset(path = "textures/github.png")]
    pub github: Handle<Image>,
}
