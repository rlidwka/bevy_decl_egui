use std::fmt::Debug;
use std::str::FromStr;
use std::vec;

use bevy::reflect::Reflect;
use jomini::{TextTape, TextToken};
use strum::{Display, EnumString, EnumVariantNames, VariantNames};

use crate::reader::binding::{Binding, BindingRef};
use crate::reader::data_model::{ResolveBinding, ResolveBindingRef, Trigger};
use crate::reader::error::Error;
use crate::reader::reader::Reader;
use crate::reader::ReadUiconf;
use crate::{const_concat, egui};

//
// Root
//

#[derive(Debug)]
pub struct Root {
    //pub windows: Vec<Window>,
    pub window: Window,
}

impl Root {
    const FIELDS: &'static [&'static str] = &["window"];

    pub fn read(data: &[u8]) -> Result<Window, Error> {
        let tape = TextTape::from_slice(data).unwrap();
        let reader = tape.utf8_reader();
        let mut window = None;

        for (key, op, value) in reader.fields() {
            let value = Reader::new(value, vec![key.read_str().into()]);
            let key = key.read_str();
            if key == "window" {
                if let Some(op) = op {
                    return Err(Error::unexpected_operator(&value, op));
                }
                if window.is_some() {
                    return Err(Error::duplicate_field(&value, "window"));
                }
                window = Some(value.read()?);
            } else {
                return Err(Error::unknown_field(&value, &key, Root::FIELDS));
            }
        }

        if let Some(window) = window {
            Ok(window)
        } else {
            let tape = TextTape::from_slice(b"a=b").unwrap();
            let reader = tape.utf8_reader();
            let dummy_value = Reader::new(reader.fields().next().unwrap().2, vec![]);
            Err(Error::missing_field(&dummy_value, "window"))
        }
    }
}

//
// Window
//

#[derive(Debug)]
pub struct Window {
    pub title: RichText,
    pub props: Vec<WindowProperty>,
    pub content: Content,
}

impl Window {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["title"],
        WindowProperty::FIELDS,
        ContentWidget::FIELDS,
    );

    pub fn show(&self, data: &mut dyn Reflect, ctx: &egui::Context) {
        let title = self.title.resolve(data).ok().unwrap_or_default();
        let mut window = egui::Window::new(title);

        for prop in self.props.iter() {
            use WindowProperty as P;
            match prop {
                P::Anchor(anchor) => {
                    window = window.anchor(anchor.align, anchor.offset);
                }
                P::TitleBar(title_bar) => {
                    if let Ok(title_bar) = title_bar.resolve(data) {
                        window = window.title_bar(title_bar);
                    }
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
                    if let Ok(resizable) = resizable.resolve(data) {
                        window = window.resizable(resizable);
                    }
                }

                // other flags
                P::Enabled(enabled) => {
                    if let Ok(enabled) = enabled.resolve(data) {
                        window = window.enabled(enabled);
                    }
                }
                P::Interactable(interactable) => {
                    if let Ok(interactable) = interactable.resolve(data) {
                        window = window.interactable(interactable);
                    }
                }
                P::Movable(movable) => {
                    if let Ok(movable) = movable.resolve(data) {
                        window = window.movable(movable);
                    }
                }
                P::Collapsible(collapsible) => {
                    if let Ok(collapsible) = collapsible.resolve(data) {
                        window = window.collapsible(collapsible);
                    }
                }
            }
        }

        window.show(ctx, |ui| {
            self.content.show(data, ui);
        });
    }
}

impl ReadUiconf for Window {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        let mut title = None;
        let mut props = vec![];
        let mut content = vec![];
        let mut last_content = None;

        for (key, value) in value.read_object()? {
            let mut should_be_on_top = false;

            if key == "title" {
                if title.is_some() { return Err(Error::duplicate_field(&value, "title")); }
                title = Some(value.read()?);
                should_be_on_top = true;
            } else if WindowProperty::FIELDS.contains(&&*key) {
                props.push(WindowProperty::read_map_value(&key, &value)?);
                should_be_on_top = true;
            } else if ContentWidget::FIELDS.contains(&&*key) {
                content.push(ContentWidget::read_map_value(&key, &value)?);
                last_content = Some(key.to_string());
            } else {
                return Err(Error::unknown_field(&value, &key, Window::FIELDS));
            }

            if should_be_on_top && last_content.is_some() {
                return Err(Error::custom(&value, format!(
                    "all window properties should be above content, but `{}` is located after `{}`",
                    key, last_content.unwrap(),
                )));
            }
        }

        let title = title.ok_or_else(|| Error::missing_field(value, "title"))?;

        Ok(Window {
            title,
            props,
            content: Content(content),
        })
    }
}

//
// WindowProperty
//

#[derive(Debug)]
pub enum WindowProperty {
    Anchor(Anchor),
    TitleBar(Binding<bool>),

    // everything related to resizing
    DefaultSize(egui::Vec2),
    MinSize(egui::Vec2),
    MaxSize(egui::Vec2),
    FixedSize(egui::Vec2),
    AutoSized,
    Resizable(Binding<bool>),

    // other flags
    Enabled(Binding<bool>),
    Interactable(Binding<bool>),
    Movable(Binding<bool>),
    Collapsible(Binding<bool>),
}

impl WindowProperty {
    const FIELDS: &'static [&'static str] = &[
        "id", "anchor", "title_bar",
        "default_size", "min_size", "max_size", "fixed_size", "auto_sized", "resizable",
        "enabled", "interactable", "movable", "collapsible",
    ];

    fn read_map_value(tag: &str, value: &Reader) -> Result<Self, Error> {
        match tag {
            "anchor"       => Ok(Self::Anchor       (value.read()?)),
            "title_bar"    => Ok(Self::TitleBar     (value.read()?)),
            "default_size" => Ok(Self::DefaultSize  (value.read::<Size<{ SIZE_ANY_DISALLOWED }>>()?.0)),
            "min_size"     => Ok(Self::MinSize      (value.read::<Size<{ SIZE_ANY_IS_ZERO    }>>()?.0)),
            "max_size"     => Ok(Self::MaxSize      (value.read::<Size<{ SIZE_ANY_IS_INF     }>>()?.0)),
            "fixed_size"   => Ok(Self::FixedSize    (value.read::<Size<{ SIZE_ANY_DISALLOWED }>>()?.0)),
            "auto_sized"   => { value.read::<Empty>()?; Ok(Self::AutoSized) },
            "resizable"    => Ok(Self::Resizable    (value.read()?)),
            "enabled"      => Ok(Self::Enabled      (value.read()?)),
            "interactable" => Ok(Self::Interactable (value.read()?)),
            "movable"      => Ok(Self::Movable      (value.read()?)),
            "collapsible"  => Ok(Self::Collapsible  (value.read()?)),
            _              => Err(Error::unknown_field(value, tag, Self::FIELDS)),
        }
    }
}

//
// Content
//

#[derive(Debug)]
pub struct Content(Vec<ContentWidget>);

impl Content {
    fn show(&self, data: &mut dyn Reflect, ui: &mut egui::Ui) {
        for widget in self.0.iter() {
            widget.show(data, ui);
        }
    }
}

impl ReadUiconf for Content {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        if value.is_scalar() {
            return Ok(Self(vec![ContentWidget::Label(Label::new(value.read()?))]));
        }

        let mut widgets = vec![];

        for (key, value) in value.read_object()? {
            widgets.push(ContentWidget::read_map_value(&key, &value)?);
        }

        Ok(Content(widgets))
    }
}

#[derive(Debug)]
pub enum ContentWidget {
    // widgets
    Button(Button),
    Label(Label),
    Separator(Separator),
    // containers
    Layout(Layout),
}

impl ContentWidget {
    const FIELDS: &'static [&'static str] = &["button", "label", "separator", "layout"];

    fn read_map_value(tag: &str, value: &Reader) -> Result<Self, Error> {
        match tag {
            "button"    => Ok(Self::Button    (value.read()?)),
            "label"     => Ok(Self::Label     (value.read()?)),
            "separator" => Ok(Self::Separator (value.read()?)),
            "layout"    => Ok(Self::Layout    (value.read()?)),
            _           => Err(Error::unknown_field(value, tag, Self::FIELDS)),
        }
    }

    fn show(&self, data: &mut dyn Reflect, ui: &mut egui::Ui) {
        match self {
            Self::Button(button)       => button.show(data, ui),
            Self::Label(label)         => label.show(data, ui),
            Self::Separator(separator) => separator.show(data, ui),
            Self::Layout(layout)       => layout.show(data, ui),
        }
    }
}

//
// Layout
//

#[derive(Debug)]
pub struct Layout {
    pub layout: egui::Layout,
    pub visible: Option<Binding<bool>>,
    pub content: Content,
}

impl Layout {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["main_dir", "main_wrap", "main_align", "main_justify", "cross_align", "cross_justify", "visible"],
        ContentWidget::FIELDS,
    );

    fn show(&self, data: &mut dyn Reflect, ui: &mut egui::Ui) {
        if let Some(visible) = &self.visible {
            if let Ok(visible) = visible.resolve(data) {
                if !visible { return; }
            }
        }

        ui.with_layout(self.layout, |ui| {
            self.content.show(data, ui);
        });
    }
}

impl ReadUiconf for Layout {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        #[derive(EnumString, EnumVariantNames, Debug, Clone, Copy)]
        #[strum(serialize_all = "snake_case")]
        enum Direction {
            LeftToRight,
            RightToLeft,
            TopDown,
            BottomUp,
        }

        impl ReadUiconf for Direction {
            fn read_uiconf(value: &Reader) -> Result<Self, Error> {
                let name = value.read_string()?;
                Self::from_str(&name).map_err(|_| {
                    Error::unknown_variant(value, &name, Self::VARIANTS)
                })
            }
        }

        impl From<Direction> for egui::Direction {
            fn from(dir: Direction) -> Self {
                match dir {
                    Direction::LeftToRight => egui::Direction::LeftToRight,
                    Direction::RightToLeft => egui::Direction::RightToLeft,
                    Direction::TopDown     => egui::Direction::TopDown,
                    Direction::BottomUp    => egui::Direction::BottomUp,
                }
            }
        }

        #[derive(EnumString, EnumVariantNames, Debug, Clone, Copy)]
        #[strum(serialize_all = "snake_case")]
        enum Align {
            Min,
            Center,
            Max,
        }

        impl ReadUiconf for Align {
            fn read_uiconf(value: &Reader) -> Result<Self, Error> {
                let name = value.read_string()?;
                Self::from_str(&name).map_err(|_| {
                    Error::unknown_variant(value, &name, Self::VARIANTS)
                })
            }
        }

        impl From<Align> for egui::Align {
            fn from(align: Align) -> Self {
                match align {
                    Align::Min    => egui::Align::Min,
                    Align::Center => egui::Align::Center,
                    Align::Max    => egui::Align::Max,
                }
            }
        }

        let mut layout = egui::Layout::default();
        let mut visible = None;
        let mut content = vec![];
        let mut last_content = None;

        for (key, value) in value.read_object()? {
            let mut is_content = false;
            match &*key {
                "main_dir"      => { layout.main_dir      = value.read::<Direction>()?.into(); }
                "main_wrap"     => { layout.main_wrap     = value.read()?; }
                "main_align"    => { layout.main_align    = value.read::<Align>()?.into(); }
                "main_justify"  => { layout.main_justify  = value.read()?; }
                "cross_align"   => { layout.cross_align   = value.read::<Align>()?.into(); }
                "cross_justify" => { layout.cross_justify = value.read()?; }
                "visible"       => { visible              = Some(value.read()?); }
                str => {
                    if ContentWidget::FIELDS.contains(&str) {
                        content.push(ContentWidget::read_map_value(str, &value)?);
                        last_content = Some(str.to_owned());
                        is_content = true;
                    } else {
                        return Err(Error::unknown_field(&value, str, Layout::FIELDS));
                    }
                }
            }

            if !is_content && last_content.is_some() {
                return Err(Error::custom(&value, format!(
                    "all layout properties should be above content, but `{}` is located after `{}`",
                    key, last_content.unwrap(),
                )));
            }
        }

        Ok(Layout {
            layout,
            visible,
            content: Content(content),
        })
    }
}

//
// Response
//

#[derive(Debug)]
pub struct Response(Vec<ResponseProperty>);

impl Response {
    fn process(&self, data: &mut dyn Reflect, mut response: egui::Response) {
        for prop in self.0.iter() {
            use ResponseProperty as P;
            match prop {
                P::Clicked(trigger) => {
                    if let Ok(clicked) = trigger.resolve_mut(data) {
                        if response.clicked() { clicked.trigger(); }
                    }
                }
                P::SecondaryClicked(trigger) => {
                    if let Ok(clicked) = trigger.resolve_mut(data) {
                        if response.secondary_clicked() { clicked.trigger(); }
                    }
                }
                P::MiddleClicked(trigger) => {
                    if let Ok(clicked) = trigger.resolve_mut(data) {
                        if response.middle_clicked() { clicked.trigger(); }
                    }
                }
                P::DoubleClicked(trigger) => {
                    if let Ok(clicked) = trigger.resolve_mut(data) {
                        if response.double_clicked() { clicked.trigger(); }
                    }
                }
                P::TripleClicked(trigger) => {
                    if let Ok(clicked) = trigger.resolve_mut(data) {
                        if response.triple_clicked() { clicked.trigger(); }
                    }
                }
                P::ClickedElsewhere(trigger) => {
                    if let Ok(clicked) = trigger.resolve_mut(data) {
                        if response.clicked_elsewhere() { clicked.trigger(); }
                    }
                }
                P::Hovered(trigger) => {
                    if let Ok(hovered) = trigger.resolve_mut(data) {
                        if response.hovered() { hovered.trigger(); }
                    }
                }
                P::Highlighted(trigger) => {
                    if let Ok(highlighted) = trigger.resolve_mut(data) {
                        if response.highlighted() { highlighted.trigger(); }
                    }
                }
                P::Changed(trigger) => {
                    if let Ok(changed) = trigger.resolve_mut(data) {
                        if response.changed() { changed.trigger(); }
                    }
                }
                P::OnHover(content) => {
                    response = response.on_hover_ui(|ui| {
                        content.show(data, ui);
                    });
                }
                P::OnDisabledHover(content) => {
                    response = response.on_disabled_hover_ui(|ui| {
                        content.show(data, ui);
                    });
                }
                P::OnHoverAtPointer(content) => {
                    response = response.on_hover_ui_at_pointer(|ui| {
                        content.show(data, ui);
                    });
                }
                P::Highlight(highlight) => {
                    if let Ok(highlight) = highlight.resolve(data) {
                        if highlight { response = response.highlight(); }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ResponseProperty {
    Clicked(BindingRef<Trigger>),
    SecondaryClicked(BindingRef<Trigger>),
    MiddleClicked(BindingRef<Trigger>),
    DoubleClicked(BindingRef<Trigger>),
    TripleClicked(BindingRef<Trigger>),
    ClickedElsewhere(BindingRef<Trigger>),
    Hovered(BindingRef<Trigger>),
    Highlighted(BindingRef<Trigger>),
    Changed(BindingRef<Trigger>),
    OnHover(Content),
    OnDisabledHover(Content),
    OnHoverAtPointer(Content),
    Highlight(Binding<bool>),
}

impl ResponseProperty {
    const FIELDS: &'static [&'static str] = &[
        "clicked", "secondary_clicked", "middle_clicked", "double_clicked", "triple_clicked", "clicked_elsewhere",
        "hovered", "highlighted", "changed", "on_hover", "on_disabled_hover", "on_hover_at_pointer", "highlight",
    ];

    fn read_map_value(tag: &str, value: &Reader) -> Result<Self, Error> {
        match tag {
            "clicked"            => Ok(Self::Clicked            (value.read()?)),
            "secondary_clicked"  => Ok(Self::SecondaryClicked   (value.read()?)),
            "middle_clicked"     => Ok(Self::MiddleClicked      (value.read()?)),
            "double_clicked"     => Ok(Self::DoubleClicked      (value.read()?)),
            "triple_clicked"     => Ok(Self::TripleClicked      (value.read()?)),
            "clicked_elsewhere"  => Ok(Self::ClickedElsewhere   (value.read()?)),
            "hovered"            => Ok(Self::Hovered            (value.read()?)),
            "highlighted"        => Ok(Self::Highlighted        (value.read()?)),
            "changed"            => Ok(Self::Changed            (value.read()?)),
            "on_hover"           => Ok(Self::OnHover            (value.read()?)),
            "on_disabled_hover"  => Ok(Self::OnDisabledHover    (value.read()?)),
            "on_hover_at_pointer"=> Ok(Self::OnHoverAtPointer   (value.read()?)),
            "highlight"          => Ok(Self::Highlight          (value.read()?)),
            _                    => Err(Error::unknown_field(value, tag, Self::FIELDS)),
        }
    }
}

//
// Anchor
//

#[derive(Debug)]
pub struct Anchor {
    pub align: egui::Align2,
    pub offset: egui::Vec2,
}

impl ReadUiconf for Anchor {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        const EXPECTED: &str = "{ align valign x y }";
        let mut seq = value.read_array()?;
        let mut align_x = seq.next().ok_or_else(|| Error::invalid_length(value, 0, EXPECTED))?.read::<Alignment>()?;
        let mut align_y = seq.next().ok_or_else(|| Error::invalid_length(value, 1, EXPECTED))?.read::<Alignment>()?;

        if align_x.can_be_horizontal() && align_y.can_be_vertical() {
            // all good
        } else if align_x.can_be_vertical() && align_y.can_be_horizontal() {
            std::mem::swap(&mut align_x, &mut align_y);
        } else {
            return Err(Error::custom(value, format!(
                "invalid alignment: `{} {}`",
                align_x.to_string(), align_y.to_string(),
            )));
        }

        let align = egui::Align2([
            match align_x {
                Alignment::Left   => egui::Align::Min,
                Alignment::Center => egui::Align::Center,
                Alignment::Right  => egui::Align::Max,
                _ => unreachable!(),
            },
            match align_y {
                Alignment::Top    => egui::Align::Min,
                Alignment::Center => egui::Align::Center,
                Alignment::Bottom => egui::Align::Max,
                _ => unreachable!(),
            },
        ]);

        let offset = if let Some(offset_x) = seq.next() {
            let offset_x = offset_x.read::<f32>()?;
            let offset_y = seq.next().ok_or_else(|| Error::invalid_length(value, 3, EXPECTED))?.read::<f32>()?;
            if seq.next().is_some() {
                return Err(Error::invalid_length(value, 5, EXPECTED));
            }
            egui::Vec2::new(offset_x, offset_y)
        } else {
            if seq.next().is_some() {
                return Err(Error::invalid_length(value, 3, EXPECTED));
            }
            egui::Vec2::ZERO
        };

        Ok(Anchor { align, offset })
    }
}

//
// RichText
//

#[derive(Debug)]
pub struct RichText {
    pub text: Binding<String>,
    pub props: Vec<RichTextProperty>,
}

impl RichText {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["text"],
        RichTextProperty::FIELDS,
    );

    pub fn new(text: Binding<String>) -> Self {
        Self { text, props: vec![] }
    }
}

impl ResolveBinding for RichText {
    type Item = egui::RichText;

    fn resolve(&self, data: &dyn Reflect) -> anyhow::Result<Self::Item> {
        let text = self.text.resolve_ref(data).cloned().unwrap_or_default();
        let mut result = egui::RichText::new(text);

        for prop in self.props.iter() {
            use RichTextProperty as P;
            match prop {
                P::Size(size) => {
                    if let Ok(size) = size.resolve(data) {
                        result = result.size(size);
                    }
                }
                P::Style(styles) => {
                    for style in styles {
                        result = match style {
                            RichTextStyle::Small         => result.text_style(egui::TextStyle::Small),
                            RichTextStyle::Body          => result.text_style(egui::TextStyle::Body),
                            RichTextStyle::Monospace     => result.text_style(egui::TextStyle::Monospace),
                            RichTextStyle::Button        => result.text_style(egui::TextStyle::Button),
                            RichTextStyle::Heading       => result.text_style(egui::TextStyle::Heading),
                            RichTextStyle::Code          => result.code(),
                            RichTextStyle::Strong        => result.strong(),
                            RichTextStyle::Weak          => result.weak(),
                            RichTextStyle::Strikethrough => result.strikethrough(),
                            RichTextStyle::Underline     => result.underline(),
                            RichTextStyle::Italics       => result.italics(),
                            RichTextStyle::Raised        => result.raised(),
                        };
                    }
                }
                P::Color(color) => {
                    if let Ok(color) = color.resolve(data) {
                        result = result.color(color_bevy_to_egui(color));
                    }
                }
                P::BackgroundColor(color) => {
                    if let Ok(color) = color.resolve(data) {
                        result = result.background_color(color_bevy_to_egui(color));
                    }
                }
                P::LineHeight(line_height) => {
                    if let Ok(line_height) = line_height.resolve(data) {
                        result = result.line_height(Some(line_height));
                    }
                }
                P::ExtraLetterSpacing(spacing) => {
                    if let Ok(spacing) = spacing.resolve(data) {
                        result = result.extra_letter_spacing(spacing);
                    }
                }
            }
        }

        Ok(result)
    }
}

impl ReadUiconf for RichText {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        if value.is_scalar() {
            return Ok(Self::new(value.read()?));
        }

        let mut text = None;
        let mut props = vec![];

        for (key, value) in value.read_object()? {
            if key == "text" {
                if text.is_some() { return Err(Error::duplicate_field(&value, "text")); }
                text = Some(value.read::<Binding<String>>()?);
            } else if RichTextProperty::FIELDS.contains(&&*key) {
                props.push(RichTextProperty::read_map_value(&key, &value)?);
            } else {
                return Err(Error::unknown_field(&value, &key, RichText::FIELDS));
            }
        }

        let text = text.ok_or_else(|| Error::missing_field(value, "text"))?;
        Ok(Self { text, props })
    }
}

//
// RichTextProperty
//

#[derive(Debug)]
pub enum RichTextProperty {
    Size(Binding<f32>),
    Style(Vec<RichTextStyle>),
    Color(Binding<bevy::prelude::Color>),
    BackgroundColor(Binding<bevy::prelude::Color>),
    LineHeight(Binding<f32>),
    ExtraLetterSpacing(Binding<f32>),
}

impl RichTextProperty {
    const FIELDS: &'static [&'static str] = &[
        "size", "style", "color", "background_color", "line_height", "extra_letter_spacing",
    ];

    fn read_map_value(tag: &str, value: &Reader) -> Result<Self, Error> {
        match tag {
            "size"                 => Ok(Self::Size               (value.read()?)),
            "extra_letter_spacing" => Ok(Self::ExtraLetterSpacing (value.read()?)),
            "line_height"          => Ok(Self::LineHeight         (value.read()?)),
            "style"                => Ok(Self::Style              (value.read()?)),
            "background_color"     => Ok(Self::BackgroundColor    (value.read::<Binding<Color>>()?.map_value(|c| c.0))),
            "color"                => Ok(Self::Color              (value.read::<Binding<Color>>()?.map_value(|c| c.0))),
            _ => Err(Error::unknown_field(value, tag, Self::FIELDS)),
        }
    }
}

//
// RichTextStyle
//

#[derive(EnumString, EnumVariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
pub enum RichTextStyle {
    Small,
    Body,
    Monospace,
    Button,
    Heading,
    Code,
    Strong,
    Weak,
    Strikethrough,
    Underline,
    Italics,
    Raised,
}

impl ReadUiconf for RichTextStyle {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        let name = value.read_string()?;
        Self::from_str(&name).map_err(|_| {
            Error::unknown_variant(value, &name, Self::VARIANTS)
        })
    }
}

//
// Button
//

#[derive(Debug)]
pub struct Button {
    pub text: RichText,
    pub small: bool,
    pub visible: Option<Binding<bool>>,
    pub props: Vec<ButtonProperty>,
    pub response: Response,
}

impl Button {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["text", "small", "visible"],
        ButtonProperty::FIELDS,
        ResponseProperty::FIELDS,
    );

    pub fn new(text: RichText) -> Self {
        Self {
            text,
            small: false,
            visible: None,
            props: vec![],
            response: Response(vec![]),
        }
    }

    fn show(&self, data: &mut dyn Reflect, ui: &mut egui::Ui) {
        if let Some(visible) = &self.visible {
            if let Ok(visible) = visible.resolve(data) {
                if !visible { return; }
            }
        }

        let text = self.text.resolve(data).ok().unwrap_or_default();
        let mut button = egui::Button::new(text);

        if self.small {
            button = button.small();
        }

        for prop in self.props.iter() {
            use ButtonProperty as P;
            button = match prop {
                P::ShortcutText(text) => {
                    if let Ok(text) = text.resolve(data) {
                        button.shortcut_text(text)
                    } else {
                        button
                    }
                },
                P::Wrap(wrap) => button.wrap(*wrap),
                P::Fill(color) => {
                    if let Ok(color) = color.resolve(data) {
                        button.fill(color_bevy_to_egui(color))
                    } else {
                        button
                    }
                }
                P::Stroke(stroke) => {
                    if let Ok(stroke) = stroke.resolve(data) {
                        button.stroke(stroke)
                    } else {
                        button
                    }
                }
                P::Sense(sense)       => button.sense(sense.0),
                P::Frame(frame)       => button.frame(*frame),
                P::MinSize(size)      => button.min_size(*size),
                P::Rounding(rounding) => button.rounding(*rounding),
                P::Selected(selected) => button.selected(*selected),
            };
        }

        self.response.process(data, ui.add(button));
    }
}

impl ReadUiconf for Button {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        if value.is_scalar() {
            return Ok(Self::new(value.read()?));
        }

        let mut text = None;
        let mut visible = None;
        let mut small = false;
        let mut props = vec![];
        let mut response = vec![];

        for (key, value) in value.read_object()? {
            match &*key {
                "text" => {
                    if text.is_some() { return Err(Error::duplicate_field(&value, "text")); }
                    text = Some(value.read()?);
                }
                "visible" => {
                    if visible.is_some() { return Err(Error::duplicate_field(&value, "visible")); }
                    visible = Some(value.read()?);
                }
                "small" => {
                    small = value.read()?;
                }
                str => {
                    if ButtonProperty::FIELDS.contains(&str) {
                        props.push(ButtonProperty::read_map_value(&key, &value)?);
                    } else if ResponseProperty::FIELDS.contains(&str) {
                        response.push(ResponseProperty::read_map_value(&key, &value)?);
                    } else {
                        return Err(Error::unknown_field(&value, &key, Button::FIELDS));
                    }
                }
            }
        }

        let text = text.ok_or_else(|| Error::missing_field(value, "text"))?;

        Ok(Button { text, visible, small, props, response: Response(response) })
    }
}

//
// ButtonProperty
//

#[derive(Debug)]
pub enum ButtonProperty {
    ShortcutText(RichText),
    Wrap(bool),
    Fill(Binding<bevy::prelude::Color>),
    Stroke(Stroke),
    Sense(Sense),
    Frame(bool),
    MinSize(egui::Vec2),
    Rounding(egui::Rounding),
    Selected(bool),
}

impl ButtonProperty {
    const FIELDS: &'static [&'static str] = &[
        "shortcut_text", "wrap", "fill", "stroke", "sense", "frame", "min_size", "rounding", "selected",
    ];

    fn read_map_value(tag: &str, value: &Reader) -> Result<Self, Error> {
        match tag {
            "shortcut_text" => Ok(Self::ShortcutText (value.read()?)),
            "wrap"          => Ok(Self::Wrap         (value.read()?)),
            "fill"          => Ok(Self::Fill         (value.read::<Binding<Color>>()?.map_value(|c| c.0))),
            "stroke"        => Ok(Self::Stroke       (value.read()?)),
            "sense"         => Ok(Self::Sense        (value.read()?)),
            "frame"         => Ok(Self::Frame        (value.read()?)),
            "min_size"      => Ok(Self::MinSize      (value.read::<Size<{ SIZE_ANY_IS_ZERO }>>()?.0)),
            "rounding"      => Ok(Self::Rounding     (value.read::<Rounding>()?.0)),
            "selected"      => Ok(Self::Selected     (value.read()?)),
            _               => Err(Error::unknown_field(value, tag, Self::FIELDS)),
        }
    }
}

//
// Label
//

#[derive(Debug)]
pub struct Label {
    pub text: RichText,
    pub visible: Option<Binding<bool>>,
    pub props: Vec<LabelProperty>,
    pub response: Response,
}

impl Label {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["text", "visible"],
        LabelProperty::FIELDS,
        ResponseProperty::FIELDS,
    );

    pub fn new(text: RichText) -> Self {
        Self {
            text,
            visible: None,
            props: vec![],
            response: Response(vec![]),
        }
    }

    fn show(&self, data: &mut dyn Reflect, ui: &mut egui::Ui) {
        if let Some(visible) = &self.visible {
            if let Ok(visible) = visible.resolve(data) {
                if !visible { return; }
            }
        }

        let text = self.text.resolve(data).ok().unwrap_or_default();
        let mut label = egui::Label::new(text);

        for prop in self.props.iter() {
            use LabelProperty as P;
            label = match prop {
                P::Wrap(wrap)         => label.wrap(*wrap),
                P::Truncate(truncate) => label.truncate(*truncate),
                P::Sense(sense)       => label.sense(sense.0),
            };
        }

        self.response.process(data, ui.add(label));
    }
}

impl ReadUiconf for Label {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        if value.is_scalar() {
            return Ok(Self::new(value.read()?));
        }

        let mut text = None;
        let mut visible = None;
        let mut props = vec![];
        let mut response = vec![];

        for (key, value) in value.read_object()? {
            if key == "text" {
                if text.is_some() { return Err(Error::duplicate_field(&value, "text")); }
                text = Some(value.read()?);
            } else if key == "visible" {
                if visible.is_some() { return Err(Error::duplicate_field(&value, "visible")); }
                visible = Some(value.read()?);
            } else if LabelProperty::FIELDS.contains(&&*key) {
                props.push(LabelProperty::read_map_value(&key, &value)?);
            } else if ResponseProperty::FIELDS.contains(&&*key) {
                response.push(ResponseProperty::read_map_value(&key, &value)?);
            } else {
                return Err(Error::unknown_field(&value, &key, Label::FIELDS));
            }
        }

        let text = text.ok_or_else(|| Error::missing_field(value, "text"))?;

        Ok(Label { text, visible, props, response: Response(response) })
    }
}

//
// LabelProperty
//

#[derive(Debug, Clone)]
pub enum LabelProperty {
    Wrap(bool),
    Truncate(bool),
    Sense(Sense),
}

impl LabelProperty {
    const FIELDS: &'static [&'static str] = &["wrap", "truncate", "sense"];

    fn read_map_value(tag: &str, value: &Reader) -> Result<Self, Error> {
        match tag {
            "wrap"     => Ok(Self::Wrap     (value.read()?)),
            "truncate" => Ok(Self::Truncate (value.read()?)),
            "sense"    => Ok(Self::Sense    (value.read()?)),
            _          => Err(Error::unknown_field(value, tag, Self::FIELDS)),
        }
    }
}

//
// Separator
//

#[derive(Debug)]
pub struct Separator {
    pub visible: Option<Binding<bool>>,
    pub props: Vec<SeparatorProperty>,
    pub response: Response,
}

impl Separator {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["visible"],
        SeparatorProperty::FIELDS,
        ResponseProperty::FIELDS,
    );

    fn show(&self, data: &mut dyn Reflect, ui: &mut egui::Ui) {
        if let Some(visible) = &self.visible {
            if let Ok(visible) = visible.resolve(data) {
                if !visible { return; }
            }
        }

        let mut separator = egui::Separator::default();

        for prop in self.props.iter() {
            use SeparatorProperty as P;
            separator = match prop {
                P::Vertical(vertical) => if *vertical {
                    separator.vertical()
                } else {
                    separator.horizontal()
                }
                P::Spacing(spacing)   => separator.spacing(*spacing),
                P::Grow(grow)         => separator.grow(*grow),
                P::Shrink(shrink)     => separator.shrink(*shrink),
            };
        }

        self.response.process(data, ui.add(separator));
    }
}

impl ReadUiconf for Separator {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        let mut visible = None;
        let mut props = vec![];
        let mut response = vec![];

        for (key, value) in value.read_object()? {
            if key == "visible" {
                if visible.is_some() { return Err(Error::duplicate_field(&value, "visible")); }
                visible = Some(value.read()?);
            } else if SeparatorProperty::FIELDS.contains(&&*key) {
                props.push(SeparatorProperty::read_map_value(&key, &value)?);
            } else if ResponseProperty::FIELDS.contains(&&*key) {
                response.push(ResponseProperty::read_map_value(&key, &value)?);
            } else {
                return Err(Error::unknown_field(&value, &key, Separator::FIELDS));
            }
        }

        Ok(Separator { visible, props, response: Response(response) })
    }
}

//
// SeparatorProperty
//

#[derive(Debug, Clone)]
pub enum SeparatorProperty {
    Vertical(bool),
    Spacing(f32),
    Grow(f32),
    Shrink(f32),
}

impl SeparatorProperty {
    const FIELDS: &'static [&'static str] = &["vertical", "spacing", "grow", "shrink"];

    fn read_map_value(tag: &str, value: &Reader) -> Result<Self, Error> {
        match tag {
            "vertical" => Ok(Self::Vertical   (value.read()?)),
            "spacing"  => Ok(Self::Spacing    (value.read()?)),
            "grow"     => Ok(Self::Grow       (value.read()?)),
            "shrink"   => Ok(Self::Shrink     (value.read()?)),
            _          => Err(Error::unknown_field(value, tag, Self::FIELDS)),
        }
    }
}

//
// Alignment
//

#[derive(EnumString, EnumVariantNames, Display, Debug, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
enum Alignment {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

impl Alignment {
    fn can_be_horizontal(self) -> bool {
        matches!(self, Alignment::Center | Alignment::Left | Alignment::Right)
    }

    fn can_be_vertical(self) -> bool {
        matches!(self, Alignment::Center | Alignment::Top | Alignment::Bottom)
    }
}

impl ReadUiconf for Alignment {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        let name = value.read_string()?;
        Self::from_str(&name).map_err(|_| {
            Error::unknown_variant(value, &name, Self::VARIANTS)
        })
    }
}

//
// Color
//

#[derive(Debug, Clone, Copy)]
pub struct Color(bevy::prelude::Color);

impl ReadUiconf for Color {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        if value.is_scalar() {
            let value: ColorName = value.read()?;
            return Ok(value.into());
        }

        const EXPECTED: &str = "{ r g b a? }";
        let mut seq = value.read_array()?;
        let r = seq.next().ok_or_else(|| Error::invalid_length(value, 0, EXPECTED))?.read::<u8>()?;
        let g = seq.next().ok_or_else(|| Error::invalid_length(value, 1, EXPECTED))?.read::<u8>()?;
        let b = seq.next().ok_or_else(|| Error::invalid_length(value, 2, EXPECTED))?.read::<u8>()?;
        let a = if let Some(a) = seq.next() {
            a.read::<u8>()?
        } else {
            u8::MAX
        };
        if seq.next().is_some() {
            return Err(Error::invalid_length(value, 5, EXPECTED));
        }
        Ok(Self(bevy::prelude::Color::rgba_u8(r, g, b, a)))
        //Ok(Self(egui::Color32::from_rgba_premultiplied(r, g, b, a)))
    }
}

//
// ColorName
//

#[derive(EnumString, EnumVariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
enum ColorName {
    Transparent,
    Black,
    DarkGray,
    Gray,
    LightGray,
    White,
    Brown,
    DarkRed,
    Red,
    LightRed,
    Yellow,
    LightYellow,
    Khaki,
    DarkGreen,
    Green,
    LightGreen,
    DarkBlue,
    Blue,
    LightBlue,
    Gold,
    DebugColor,
    TemporaryColor,
}

impl ReadUiconf for ColorName {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        let name = value.read_string()?;
        Self::from_str(&name).map_err(|_| {
            Error::unknown_variant(value, &name, Self::VARIANTS)
        })
    }
}

impl From<ColorName> for egui::Color32 {
    fn from(name: ColorName) -> egui::Color32 {
        match name {
            ColorName::Transparent    => egui::Color32::TRANSPARENT,
            ColorName::Black          => egui::Color32::BLACK,
            ColorName::DarkGray       => egui::Color32::DARK_GRAY,
            ColorName::Gray           => egui::Color32::GRAY,
            ColorName::LightGray      => egui::Color32::LIGHT_GRAY,
            ColorName::White          => egui::Color32::WHITE,
            ColorName::Brown          => egui::Color32::BROWN,
            ColorName::DarkRed        => egui::Color32::DARK_RED,
            ColorName::Red            => egui::Color32::RED,
            ColorName::LightRed       => egui::Color32::LIGHT_RED,
            ColorName::Yellow         => egui::Color32::YELLOW,
            ColorName::LightYellow    => egui::Color32::LIGHT_YELLOW,
            ColorName::Khaki          => egui::Color32::KHAKI,
            ColorName::DarkGreen      => egui::Color32::DARK_GREEN,
            ColorName::Green          => egui::Color32::GREEN,
            ColorName::LightGreen     => egui::Color32::LIGHT_GREEN,
            ColorName::DarkBlue       => egui::Color32::DARK_BLUE,
            ColorName::Blue           => egui::Color32::BLUE,
            ColorName::LightBlue      => egui::Color32::LIGHT_BLUE,
            ColorName::Gold           => egui::Color32::GOLD,
            ColorName::DebugColor     => egui::Color32::DEBUG_COLOR,
            ColorName::TemporaryColor => egui::Color32::TEMPORARY_COLOR,
        }
    }
}

impl From<ColorName> for Color {
    fn from(name: ColorName) -> Color {
        let color: egui::Color32 = name.into();
        Color(color_egui_to_bevy(color))
    }
}

//
// Stroke
//

#[derive(Debug)]
pub struct Stroke {
    pub width: Binding<f32>,
    pub color: Binding<bevy::prelude::Color>,
}

impl ResolveBinding for Stroke {
    type Item = egui::Stroke;

    fn resolve(&self, data: &dyn Reflect) -> anyhow::Result<Self::Item> {
        let width = self.width.resolve(data).unwrap_or_default();
        let color = self.color.resolve(data).unwrap_or_default();
        Ok(egui::Stroke::new(width, color_bevy_to_egui(color)))
    }
}

impl ReadUiconf for Stroke {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        const EXPECTED: &str = "{ width color } or none";

        if let Ok(str) = value.read_string() {
            if str == "none" {
                let stroke = egui::Stroke::NONE;
                return Ok(Self { width: Binding::Value(stroke.width), color: Binding::Value(color_egui_to_bevy(stroke.color)) });
            }
        }

        let mut seq = value.read_array()?;
        let width = seq.next().ok_or_else(|| Error::invalid_length(value, 0, EXPECTED))?.read()?;
        let color = seq.next().ok_or_else(|| Error::invalid_length(value, 1, EXPECTED))?.read::<Binding<Color>>()?.map_value(|x| x.0);
        if seq.next().is_some() {
            return Err(Error::invalid_length(value, 3, EXPECTED));
        }
        Ok(Self { width, color })
    }
}

//
// Rounding
//

#[derive(Debug, Clone, Copy)]
pub struct Rounding(pub egui::Rounding);

impl ReadUiconf for Rounding {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        const EXPECTED: &str = "{ top-left top-right bottom-right bottom-left }";

        if let Ok(str) = value.read_string() {
            if str == "none" {
                return Ok(Rounding(egui::Rounding::ZERO));
            } else {
                return Ok(Rounding(egui::Rounding::same(value.read()?)));
            }
        }

        let mut seq = value.read_array()?;

        // same semantics as in CSS
        let top_left     = seq.next().ok_or_else(|| Error::invalid_length(value, 0, EXPECTED))?.read::<f32>()?;
        let top_right    = seq.next().ok_or_else(|| Error::invalid_length(value, 1, EXPECTED))?.read::<f32>().unwrap_or(top_left);
        let bottom_right = seq.next().ok_or_else(|| Error::invalid_length(value, 2, EXPECTED))?.read::<f32>().unwrap_or(top_left);
        let bottom_left  = seq.next().ok_or_else(|| Error::invalid_length(value, 3, EXPECTED))?.read::<f32>().unwrap_or(top_right);

        if seq.next().is_some() {
            return Err(Error::invalid_length(value, 5, EXPECTED));
        }

        Ok(Rounding(egui::Rounding {
            nw: top_left,
            ne: top_right,
            se: bottom_right,
            sw: bottom_left,
        }))
    }
}

//
// Sense
//

#[derive(Debug, Clone)]
pub struct Sense(pub egui::Sense);

impl ReadUiconf for Sense {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        let sense = if let Ok(str) = value.read_string() {
            #[derive(EnumString, EnumVariantNames, Debug, Clone, Copy)]
            #[strum(serialize_all = "snake_case")]
            enum SenseKind {
                Hover,
                FocusableNoninteractive,
                Click,
                Drag,
                ClickAndDrag,
            }

            let sense_kind = SenseKind::from_str(&str).map_err(|_| {
                Error::unknown_variant(value, &str, SenseKind::VARIANTS)
            })?;

            match sense_kind {
                SenseKind::Hover                   => egui::Sense::hover(),
                SenseKind::FocusableNoninteractive => egui::Sense::focusable_noninteractive(),
                SenseKind::Click                   => egui::Sense::click(),
                SenseKind::Drag                    => egui::Sense::drag(),
                SenseKind::ClickAndDrag            => egui::Sense::click_and_drag(),
            }
        } else {
            #[derive(EnumString, EnumVariantNames, Debug, Clone, Copy)]
            #[strum(serialize_all = "snake_case")]
            enum SenseType {
                Click,
                Drag,
                Focusable,
            }

            impl ReadUiconf for SenseType {
                fn read_uiconf(value: &Reader) -> Result<Self, Error> {
                    let name = value.read_string()?;
                    Self::from_str(&name).map_err(|_| {
                        Error::unknown_variant(value, &name, Self::VARIANTS)
                    })
                }
            }

            let mut sense = egui::Sense::hover();
            for sense_type in value.read_array()? {
                match sense_type.read::<SenseType>()? {
                    SenseType::Click     => sense.click = true,
                    SenseType::Drag      => sense.drag = true,
                    SenseType::Focusable => sense.focusable = true,
                }
            }
            sense
        };

        Ok(Sense(sense))
    }
}

//
// Size
//

const SIZE_ANY_IS_ZERO: u8 = 0;
const SIZE_ANY_IS_INF: u8 = 1;
const SIZE_ANY_DISALLOWED: u8 = 2;
struct Size<const ANY: u8>(egui::Vec2);

impl<const ANY: u8> ReadUiconf for Size<ANY> {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        const EXPECTED: &str = "{ x y }";
        let mut seq = value.read_array()?;

        if ANY == SIZE_ANY_DISALLOWED {
            let x = seq.next().ok_or_else(|| Error::invalid_length(value, 0, EXPECTED))?.read::<f32>()?;
            let y = seq.next().ok_or_else(|| Error::invalid_length(value, 1, EXPECTED))?.read::<f32>()?;
            if seq.next().is_some() {
                return Err(Error::invalid_length(value, 3, EXPECTED));
            }
            Ok(Size(egui::Vec2::new(x, y)))
        } else {
            let x = seq.next().ok_or_else(|| Error::invalid_length(value, 0, EXPECTED))?.read::<AnyOrF32>()?.0;
            let y = seq.next().ok_or_else(|| Error::invalid_length(value, 1, EXPECTED))?.read::<AnyOrF32>()?.0;
            if seq.next().is_some() {
                return Err(Error::invalid_length(value, 3, EXPECTED));
            }
            Ok(Size(egui::Vec2::new(
                x.unwrap_or(if ANY == SIZE_ANY_IS_ZERO { 0.0 } else { f32::INFINITY }),
                y.unwrap_or(if ANY == SIZE_ANY_IS_ZERO { 0.0 } else { f32::INFINITY }),
            )))
        }
    }
}

//
// AnyOrF32
//

struct AnyOrF32(Option<f32>);

impl ReadUiconf for AnyOrF32 {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        let scalar = value.read_scalar()?;
        if scalar.as_bytes() == b"any" {
            Ok(AnyOrF32(None))
        } else {
            Ok(AnyOrF32(Some(f32::read_uiconf(value)?)))
        }
    }
}

//
// Empty
//

// This struct only allows `{}` and nothing else.
struct Empty;

impl ReadUiconf for Empty {
    fn read_uiconf(value: &Reader) -> Result<Self, Error> {
        match value.token() {
            TextToken::Array { .. } => Ok(Empty),
            TextToken::Object { .. } => Ok(Empty),
            _ => Err(Error::invalid_type(value, value.token_type(), "{}")),
        }
    }
}

//
// Conversions
//

fn color_egui_to_bevy(color: egui::Color32) -> bevy::prelude::Color {
    bevy::prelude::Color::rgba_u8(color.r(), color.g(), color.b(), color.a())
}

fn color_bevy_to_egui(color: bevy::prelude::Color) -> egui::Color32 {
    let r = (color.r() * 255.) as u8;
    let g = (color.g() * 255.) as u8;
    let b = (color.b() * 255.) as u8;
    let a = (color.a() * 255.) as u8;
    egui::Color32::from_rgba_premultiplied(r, g, b, a)
}
