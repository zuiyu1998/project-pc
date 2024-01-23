use bevy::prelude::*;
use bevy_editor_pls::prelude::*;

pub struct InternalEditorPlugin;

impl Plugin for InternalEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EditorPlugin::default());
    }
}
