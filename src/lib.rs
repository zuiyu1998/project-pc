#![allow(clippy::type_complexity)]

mod app_splash;
mod audio;
mod player;
mod ui;
mod voxel;

#[cfg(feature = "dev")]
mod editor;

use std::f32::consts::TAU;

use bevy::app::App;
use bevy::prelude::*;
use bevy_atmosphere::prelude::*;
use bevy_xpbd_3d::prelude::*;

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
            .add_plugins((
                PhysicsPlugins::default(),
                app_splash::SplashPlugin,
                audio::InternalAudioPlugin,
                ui::UiPlugin,
                voxel::VoxelPlugin,
                player::CharacterControllerPlugin,
                AtmospherePlugin,
            ))
            .add_systems(Startup, setup_environment);

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

pub fn setup_environment(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection {
                fov: TAU / 4.6,
                ..default()
            }),
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        AtmosphereCamera::default(),
        Name::new("Camera"),
    ));

    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        Name::new("Sun"),
    ));
}
