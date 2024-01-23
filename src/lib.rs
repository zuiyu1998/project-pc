#![allow(clippy::type_complexity)]

mod app_splash;
mod audio;
mod ui;

#[cfg(feature = "dev")]
mod editor;

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
        app.add_state::<AppState>().add_plugins((
            app_splash::SplashPlugin,
            audio::InternalAudioPlugin,
            ui::UiPlugin,
        ));

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
