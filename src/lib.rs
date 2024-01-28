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
use bevy::window::{CursorGrabMode, PrimaryWindow, WindowMode};
use bevy_atmosphere::prelude::*;
use bevy_editor_pls::editor::EditorEvent;
use bevy_xpbd_3d::prelude::*;
use player::*;

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

        app.add_systems(Update, handle_inputs);

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

pub fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut editor_events: EventWriter<bevy_editor_pls::editor::EditorEvent>,
) {
    editor_events.send(bevy_editor_pls::editor::EditorEvent::Toggle { now_active: false });

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
        player::CharacterControllerCamera,
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

    // Logical Player
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Capsule {
                radius: 0.4,
                depth: 1.0,
                ..default()
            })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 2.0, 0.0),
            ..default()
        },
        CharacterControllerBundle::new(
            Collider::capsule(1., 0.4),
            CharacterController {
                is_flying: true,
                ..default()
            },
        ),
        Name::new("Player"),
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

fn handle_inputs(
    mut editor_events: EventReader<bevy_editor_pls::editor::EditorEvent>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut controller_query: Query<&mut CharacterController>,
    key: Res<Input<KeyCode>>,
    // mouse_input: Res<Input<MouseButton>>,
) {
    let mut window = window_query.single_mut();

    // Toggle MouseGrab
    for event in editor_events.read() {
        if let EditorEvent::Toggle { now_active } = *event {
            let playing = !now_active;
            window.cursor.grab_mode = if playing {
                CursorGrabMode::Locked
            } else {
                CursorGrabMode::None
            };
            window.cursor.visible = !playing;
            for mut controller in &mut controller_query {
                controller.enable_input = playing;
            }
        }
    }

    // Toggle Fullscreen
    if key.just_pressed(KeyCode::F11)
        || (key.pressed(KeyCode::AltLeft) && key.just_pressed(KeyCode::Return))
    {
        window.mode = if window.mode != WindowMode::Fullscreen {
            WindowMode::Fullscreen
        } else {
            WindowMode::Windowed
        };
    }
}
