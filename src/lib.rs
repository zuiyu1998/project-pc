#![allow(clippy::type_complexity)]

mod app_splash;
mod audio;

#[cfg(feature = "dev")]
mod editor;

use crate::app_splash::SplashPlugin;
use crate::audio::InternalAudioPlugin;

use bevy::app::App;
use bevy::prelude::*;

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    #[default]
    Splash,
    MainMenu,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<AppState>()
            .add_plugins((SplashPlugin, InternalAudioPlugin));

        #[cfg(feature = "dev")]
        {
            app.add_plugins((editor::InternalEditorPlugin,));
        }

        #[cfg(not(feature = "dev"))]
        {
            use bevy_egui::EguiPlugin;

            app.add_plugins((EguiPlugin,));
        }
    }
}
