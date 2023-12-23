use std::any::Any;

use bevy::asset::{AssetLoader, AsyncReadExt};
use bevy::prelude::*;
use bevy::utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};
use egui::Widget;

use crate::{egui, model, Label};

#[derive(Asset, TypePath, Clone, Debug)]
pub struct EguiAsset<L: Label>{
    pub window: model::Window,
    hash: egui::Id,
    _labels: std::marker::PhantomData<L>,
}

impl<L: Label> EguiAsset<L> {
    pub fn prepare(&self) -> EguiWidgetBuilder<L> {
        EguiWidgetBuilder::new(self)
    }

    pub fn show(&self, ctx: &mut egui::Context) {
        self.prepare().show(ctx);
    }
}

pub trait LabelToId<L: Label> {
    fn to_id(&self) -> egui::Id;
}

impl LabelToId<String> for &str {
    fn to_id(&self) -> egui::Id {
        // assert_eq!(egui::Id::new("test"), egui::Id::new("test".to_owned()));
        egui::Id::new(*self)
    }
}

impl<T: Label> LabelToId<T> for T {
    fn to_id(&self) -> egui::Id {
        egui::Id::new(self)
    }
}

pub struct EguiWidgetBuilder<'a, L: Label> {
    asset: &'a EguiAsset<L>,
    commands: HashMap<egui::Id, WidgetCommand>,
}

impl<'a, L: Label> EguiWidgetBuilder<'a, L> {
    pub fn new(asset: &'a EguiAsset<L>) -> Self {
        Self { asset, commands: Default::default() }
    }

    pub fn init_with<W: egui::Widget + 'static>(&mut self, name: impl LabelToId<L>, widget: W) {
        let command = self.commands.entry(name.to_id()).or_default();
        command.widget = Some(Box::new(widget));
    }

    pub fn after_init<W: egui::Widget + 'static>(&mut self, name: impl LabelToId<L>, f: impl FnOnce(W) -> W + 'static) {
        let command = self.commands.entry(name.to_id()).or_default();
        command.map = Some(Box::new(WidgetCommandMapFn { f: Box::new(f) }));
    }

    pub fn on_response(&mut self, name: impl LabelToId<L>, f: impl FnOnce(egui::Response) + 'static) {
        let command = self.commands.entry(name.to_id()).or_default();
        command.response = Some(Box::new(f));
    }

    pub fn show(mut self, ctx: &mut egui::Context) {
        let desc = &self.asset.window;
        let mut window = egui::Window::new(desc.title.0.clone()).id(self.asset.hash);

        for prop in desc.props.iter() {
            use model::WindowProperty as P;
            match prop {
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
                for content in &desc.content {
                    show_content(ui, content, &mut self.commands);
                }
            });
    }
}

fn show_content(
    ui: &mut egui::Ui,
    content: &model::Content,
    commands: &mut HashMap<egui::Id, WidgetCommand>,
) {
    match content {
        model::Content::Button(desc) => {
            display_widget::<egui::Button>(
                ui, desc.name.as_ref(), commands,
                || egui::Button::new(desc.text.0.clone()),
                |mut button| {
                    if desc.small { button = button.small(); }
                    for prop in desc.props.iter() {
                        use model::ButtonProperty as P;
                        button = match prop {
                            P::ShortcutText(text) => button.shortcut_text(text.0.clone()),
                            P::Wrap(wrap)         => button.wrap(*wrap),
                            P::Fill(color)        => button.fill(*color),
                            P::Stroke(stroke)     => button.stroke(*stroke),
                            P::Sense(sense)       => button.sense(sense.0),
                            P::Frame(frame)       => button.frame(*frame),
                            P::MinSize(size)      => button.min_size(*size),
                            P::Rounding(rounding) => button.rounding(*rounding),
                            P::Selected(selected) => button.selected(*selected),
                        };
                    }
                    button
                },
            );
        }

        model::Content::Label(desc) => {
            display_widget::<egui::Label>(
                ui, desc.name.as_ref(), commands,
                || egui::Label::new(desc.text.0.clone()),
                |mut label| {
                    for prop in desc.props.iter() {
                        use model::LabelProperty as P;
                        label = match prop {
                            P::Wrap(wrap)         => label.wrap(*wrap),
                            P::Truncate(truncate) => label.truncate(*truncate),
                            P::Sense(sense)       => label.sense(sense.0),
                        }
                    }
                    label
                },
            );
        }

        model::Content::Separator(desc) => {
            display_widget::<egui::Separator>(
                ui, desc.name.as_ref(), commands,
                egui::Separator::default,
                |mut separator| {
                    if let Some(is_horizontal_line) = desc.is_horizontal {
                        if is_horizontal_line {
                            separator = separator.horizontal();
                        } else {
                            separator = separator.vertical();
                        }
                    }
                    for prop in desc.props.iter() {
                        use model::SeparatorProperty as P;
                        separator = match prop {
                            P::Spacing(spacing) => separator.spacing(*spacing),
                            P::Grow(grow)       => separator.grow(*grow),
                            P::Shrink(shrink)   => separator.shrink(*shrink),
                        }
                    }
                    separator
                },
            );
        }

        model::Content::Layout(desc) => {
            ui.with_layout(desc.layout, |ui| {
                for content in &desc.content {
                    show_content(ui, content, commands);
                }
            });
        }
    }
}

fn display_widget<W: Widget + 'static>(
    ui: &mut egui::Ui,
    name: Option<&model::Name>,
    commands: &mut HashMap<egui::Id, WidgetCommand>,
    default: impl FnOnce() -> W,
    attach_props: impl FnOnce(W) -> W,
) {
    fn get_command_for_widget_with_id<'a>(
        commands: &'a mut HashMap<egui::Id, WidgetCommand>,
        name: Option<&model::Name>,
    ) -> Option<&'a mut WidgetCommand> {
        let Some(name) = name else { return None; };
        let Some(command) = commands.get_mut(name.id.as_ref().unwrap()) else { return None; };
        Some(command)
    }

    fn get_default_widget_with_id<W: Widget + 'static>(
        command: &mut WidgetCommand,
        name: Option<&model::Name>,
    ) -> Option<W> {
        let Some(w) = command.widget.take() else { return None; };

        let Ok(w) = w.downcast::<W>().map_err(|_| {
            bevy::log::info!(
                "type mismatch for widget `{:?}`",
                name.as_ref().map(|name| name.str.as_str()).unwrap_or(""),
            );
        }) else { return None; };

        Some(*w)
    }

    let mut command = get_command_for_widget_with_id(commands, name);

    let mut widget = None;
    if command.is_some() {
        widget = get_default_widget_with_id::<W>(
            command.as_mut().unwrap(),
            name,
        );
    }

    widget = Some(widget.unwrap_or_else(default));
    widget = Some(attach_props(widget.unwrap()));

    if let Some(map) = command.as_mut().and_then(|command| command.map.take()) {
        if let Ok(map) = map.downcast::<WidgetCommandMapFn<W>>() {
            let w = widget.take().unwrap();
            let mut value: Option<Box<dyn Any>> = Some(Box::new(w));
            map.exec(&mut value);
            let w = value.unwrap().downcast::<W>().unwrap();
            widget = Some(*w);
        } else {
            bevy::log::info!(
                "type mismatch for widget `{:?}`",
                name.as_ref().map(|name| name.str.as_str()).unwrap_or(""),
            );
        }
    }

    let response = ui.add(widget.unwrap());

    if let Some(handler) = command.as_mut().and_then(|command| command.response.take()) {
        handler(response);
    }
}

#[derive(Default)]
struct WidgetCommand {
    widget: Option<Box<dyn Any>>,
    map: Option<Box<dyn WidgetCommandMap>>,
    response: Option<Box<dyn FnOnce(egui::Response)>>,
}

trait WidgetCommandMap: Downcast {
    fn exec(self, widget: &mut Option<Box<dyn Any>>);
}

impl_downcast!(WidgetCommandMap);

struct WidgetCommandMapFn<W: Widget + 'static> {
    f: Box<dyn FnOnce(W) -> W>,
}

impl<W: Widget + 'static> WidgetCommandMap for WidgetCommandMapFn<W> {
    fn exec(self, widget_box: &mut Option<Box<dyn Any>>) {
        let widget = widget_box.take().unwrap();
        let widget = widget.downcast::<W>().unwrap();
        let widget = (self.f)(*widget);
        let widget = Box::new(widget) as Box<dyn Any>;
        *widget_box = Some(widget);
    }
}

pub struct EguiAssetLoader<L> {
    _label: std::marker::PhantomData<L>,
}

impl<L: Label> AssetLoader for EguiAssetLoader<L> {
    type Asset = EguiAsset<L>;
    type Error = anyhow::Error;
    type Settings = EguiAssetLoaderSettings;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        settings: &'a Self::Settings,
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            if settings.version == 0 {
                return Err(anyhow::anyhow!("
Please use `asset_server.load_uiconf` instead of `asset_server.load`.

Add `use bevy_uiconf_egui::AssetServerExt;` to access it."));
            }

            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).await?;
            let file: model::Root = jomini::text::de::from_utf8_slice(&buffer)?;
            let file = file.assign_ids::<L>();

            Ok(EguiAsset {
                window: file.window,
                hash: egui::Id::new((load_context.asset_path(), /*settings.version*/)),
                _labels: Default::default(),
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gui"]
    }
}

impl<L> Default for EguiAssetLoader<L> {
    fn default() -> Self {
        Self { _label: Default::default() }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct EguiAssetLoaderSettings {
    pub version: u32,
}
