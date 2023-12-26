use std::time::Duration;

use bevy::winit::UpdateMode;
use bevy::{input::common_conditions::input_toggle_active, winit::WinitSettings};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_uiconf_egui::reader::data_model::DataModel;
use bevy_uiconf_egui::{egui, UiconfPlugin, UiconfWindow, AssetServerExt};
use serde::{Deserialize, Serialize};

#[derive(Resource, Default)]
struct MyWindow {
    handle: Handle<UiconfWindow<MyWidgets>>,
}

#[derive(Serialize, Deserialize, TypePath, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
enum MyWidgets {
    MyLabel,
}
//type MyWidgets = String;

fn main() {
    // For hot-reloading of the assets during testing, I want bevy to check
    // for changes often, but not eat the entire CPU as it usually does.
    // Using `Reactive` and checking every 100ms seems to be a good value for this.
    let wait = Duration::from_secs_f32(0.1);

    App::new()
        .add_plugins((
            DefaultPlugins,
            WorldInspectorPlugin::new()
                .run_if(input_toggle_active(false, KeyCode::F12)),
            UiconfPlugin::<MyWidgets>::new(),
        ))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Reactive { wait },
            unfocused_mode: UpdateMode::Reactive { wait },
            ..Default::default()
        })
        .add_systems(Startup, initialize_uiconf_assets)
        .add_systems(Update, display_custom_window)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, bevy_uiconf_egui::clear_egui_state_on_reload::<MyWidgets>)
        .run();
}

fn initialize_uiconf_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load_uiconf("gui/window.gui");
    commands.insert_resource(MyWindow { handle });
}

fn display_custom_window(
    uiconf_assets: Res<Assets<UiconfWindow<MyWidgets>>>,
    my_window: Res<MyWindow>,
    mut egui_contexts: EguiContexts,
) {
    let Some(window) = uiconf_assets.get(&my_window.handle) else { return; };

    let mut data = DataModel::new();
    data.set("text", "qwertyuio".to_string());
    data.set("color", egui::Color32::RED);
    data.set("true", true);
    data.set("false", false);

    window.show(&mut data, egui_contexts.ctx_mut());
}
