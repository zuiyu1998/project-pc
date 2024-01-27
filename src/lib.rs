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
            .add_systems(PostUpdate, gizmo_sys.after(PhysicsSet::Sync))
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

fn gizmo_sys(mut gizmo: Gizmos, mut gizmo_config: ResMut<GizmoConfig>) {
    gizmo_config.depth_bias = -1.; // always in front

    // World Basis Axes
    let n = 5;
    gizmo.line(Vec3::ZERO, Vec3::X * 2. * n as f32, Color::RED);
    gizmo.line(Vec3::ZERO, Vec3::Y * 2. * n as f32, Color::GREEN);
    gizmo.line(Vec3::ZERO, Vec3::Z * 2. * n as f32, Color::BLUE);

    let color = Color::GRAY;
    for x in -n..=n {
        gizmo.ray(
            Vec3::new(x as f32, 0., -n as f32),
            Vec3::Z * n as f32 * 2.,
            color,
        );
    }
    for z in -n..=n {
        gizmo.ray(
            Vec3::new(-n as f32, 0., z as f32),
            Vec3::X * n as f32 * 2.,
            color,
        );
    }
}
