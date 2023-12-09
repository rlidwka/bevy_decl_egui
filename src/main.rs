use std::time::Duration;

use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_inspector_egui::egui::Layout;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use loader::{EguiAsset, EguiAssetLoader};

mod const_concat;
mod loader;
mod model;

// re-export egui
pub use bevy_inspector_egui::egui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(
            WorldInspectorPlugin::new()
                .run_if(input_toggle_active(false, KeyCode::F12))
        )
        .init_asset::<EguiAsset>()
        .init_asset_loader::<EguiAssetLoader>()
        .insert_resource(WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Reactive {
                wait: Duration::from_secs_f32(0.1),
            },
            unfocused_mode: bevy::winit::UpdateMode::Reactive {
                wait: Duration::from_secs_f32(0.1),
            },
            ..Default::default()
        })
        .add_systems(Startup, load_ui_assets)
        .add_systems(Update, display_ui)
        .add_systems(PreUpdate, clear_egui_state)
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

#[derive(Resource, Default)]
struct State {
    handle: Handle<EguiAsset>,
}

fn load_ui_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(State {
        handle: asset_server.load("gui/window.gui"),
    });
}

fn clear_egui_state(
    mut events: EventReader<AssetEvent<EguiAsset>>,
    mut egui_contexts: EguiContexts
) {
    if !events.is_empty() {
        egui_contexts.ctx_mut().memory_mut(|mem| *mem = Default::default());
    }
    events.clear();
}

fn display_ui(mut egui_contexts: EguiContexts, state: ResMut<State>, custom_assets: Res<Assets<EguiAsset>>) {
    let ctx = egui_contexts.ctx_mut();

    let Some(window) = custom_assets.get(&state.handle) else { return; };
    let desc = &window.0;

    let mut window = egui::Window::new(desc.title.0.clone());

    for prop in desc.props.iter() {
        use model::WindowProperty as P;
        match prop {
            P::Id(id) => {
                window = window.id(*id);
            }
            P::Anchor(anchor) => {
                window = window.anchor(anchor.align, anchor.offset);
            }
            P::TitleBar(title_bar) => {
                window = window.title_bar(*title_bar);
            }

            // everything related to resizing
            P::DefaultSize(size) => {
                window = window.default_size(*size);
            }
            P::MinSize(size) => {
                // TODO: simplify after updating to egui 0.24
                window = window.resize(|resize| resize.min_size(*size));
            }
            P::MaxSize(size) => {
                // TODO: simplify after updating to egui 0.24
                window = window.resize(|resize| resize.max_size(*size));
            }
            P::FixedSize(size) => {
                window = window.fixed_size(*size);
            }
            P::AutoSized => {
                window = window.auto_sized();
            }
            P::Resizable(resizable) => {
                window = window.resizable(*resizable);
            }

            // other flags
            P::Enabled(enabled) => {
                window = window.enabled(*enabled);
            }
            P::Interactable(interactable) => {
                window = window.interactable(*interactable);
            }
            P::Movable(movable) => {
                window = window.movable(*movable);
            }
            P::Collapsible(collapsible) => {
                window = window.collapsible(*collapsible);
            }
        }
    }

    window
        .show(ctx, |ui| {
            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                for content in &desc.content {
                    match content {
                        model::Content::Label(label) => {
                            ui.label(&label.text);
                        }
                        model::Content::Separator => {
                            ui.separator();
                        }
                    }
                }
            });
        });
}
