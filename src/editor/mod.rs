use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_editor_pls::{
    controls::{Action, Binding, BindingCondition, Button, EditorControls, UserInput},
    prelude::*,
};

pub struct InternalEditorPlugin;

impl Plugin for InternalEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            EditorPlugin::default(),
            FrameTimeDiagnosticsPlugin,
            EntityCountDiagnosticsPlugin,
        ))
        .insert_resource(res_editor_controls());
    }
}

fn res_editor_controls() -> EditorControls {
    let mut editor_controls = EditorControls::default_bindings();
    editor_controls.unbind(Action::PlayPauseEditor);

    editor_controls.insert(
        Action::PlayPauseEditor,
        Binding {
            input: UserInput::Single(Button::Keyboard(KeyCode::Escape)),
            conditions: vec![BindingCondition::ListeningForText(false)],
        },
    );

    editor_controls
}
