use std::time::Duration;

use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_uiconf_egui::reader::data_model::Trigger;
use bevy_uiconf_egui::{AssetServerExt, UiconfPlugin, UiconfWindow};

#[derive(Resource, Default)]
struct MyWindow {
    handle: Handle<UiconfWindow>,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
struct DataModel {
    text: String,
    color: Color,
    xtrue: bool,
    xfalse: bool,
    trigger: Trigger,
}

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
            UiconfPlugin::new(),
        ))
        .register_type::<DataModel>()
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Reactive { wait },
            unfocused_mode: UpdateMode::Reactive { wait },
            ..Default::default()
        })
        .insert_resource(DataModel {
            text: "qwertyuio".to_string(),
            color: Color::RED,
            xtrue: true,
            xfalse: false,
            trigger: Trigger::default()
        })
        .add_systems(Startup, initialize_uiconf_assets)
        .add_systems(Update, display_custom_window)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, bevy_uiconf_egui::clear_egui_state_on_reload)
        .run();
}

fn initialize_uiconf_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load_uiconf("gui/window.gui");
    commands.insert_resource(MyWindow { handle });
}

fn display_custom_window(
    mut data: ResMut<DataModel>,
    uiconf_assets: Res<Assets<UiconfWindow>>,
    my_window: Res<MyWindow>,
    mut egui_contexts: EguiContexts,
) {
    let Some(window) = uiconf_assets.get(&my_window.handle) else { return; };

    /*let mut data = DataModel::new();
    data.set("text", "qwertyuio".to_string());
    data.set("color", egui::Color32::RED);
    data.set("true", true);
    data.set("false", false);*/

    window.show(data.as_reflect_mut(), egui_contexts.ctx_mut());

    if data.trigger.check_reset() {
        println!("triggered!");
    }
}
