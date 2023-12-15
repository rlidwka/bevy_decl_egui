use std::fmt::Debug;

use serde::de::{Error, MapAccess, SeqAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};

use crate::loader::LabelToId;
use crate::{const_concat, egui};

//
// Root
//

#[derive(Debug, Clone)]
pub struct Root {
    //pub windows: Vec<Window>,
    pub window: Window,
}

impl Root {
    const FIELDS: &'static [&'static str] = &["window"];

    pub fn assign_ids<L: crate::Label>(mut self) -> Self {
        for mut content in self.window.content.iter_mut() {
            let mut name = match &mut content {
                Content::Button(desc)    => &mut desc.name,
                Content::Label(desc)     => &mut desc.name,
                Content::Separator(desc) => &mut desc.name,
            };

            let Some(name) = &mut name else { continue; };

            let parsed: Result<L, serde_value::DeserializerError> = L::deserialize(serde_value::ValueDeserializer::new(
                serde_value::Value::String(name.str.clone()),
            ));
            let Ok(parsed) = parsed else {
                bevy::log::warn!("invalid widget name: `{}`", &name.str);
                continue;
            };

            name.id = Some(parsed.to_id());
        }
        self
    }
}

impl<'de> Deserialize<'de> for Root {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Root;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("{ window = .. }")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                /*let mut windows = vec![];
                while let Some(str) = map.next_key::<&str>()? {
                    match str {
                        "window" => { windows.push(map.next_value()?); }
                        _ => { return Err(Error::unknown_field(str, Root::FIELDS)); }
                    }
                }
                Ok(Root { windows })*/

                let mut window = None;

                while let Some(str) = map.next_key::<&str>()? {
                    match str {
                        "window" => {
                            if window.is_some() { return Err(Error::duplicate_field("window")); }
                            window = Some(map.next_value()?);
                        }
                        _ => { return Err(Error::unknown_field(str, Root::FIELDS)); }
                    }
                }

                let window = window.ok_or_else(|| Error::missing_field("window"))?;
                Ok(Root { window })
            }
        }

        deserializer.deserialize_struct("root", Self::FIELDS, TVisitor)
    }
}

//
// Window
//

#[derive(Debug, Clone)]
pub struct Window {
    pub title: RichText,
    pub props: Vec<WindowProperty>,
    pub content: Vec<Content>,
}

impl Window {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["title"],
        WindowProperty::FIELDS,
        Content::FIELDS,
    );
}

impl<'de> Deserialize<'de> for Window {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Window;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("{ title = .., .. }")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut title = None;
                let mut props = vec![];
                let mut content = vec![];
                let mut last_content = None;

                while let Some(str) = map.next_key::<&str>()? {
                    let mut should_be_on_top = false;

                    if str == "title" {
                        title = Some(map.next_value()?);
                        should_be_on_top = true;
                    } else if WindowProperty::FIELDS.contains(&str) {
                        props.push(WindowProperty::deserialize_map_value(str, &mut map)?);
                        should_be_on_top = true;
                    } else if Content::FIELDS.contains(&str) {
                        content.push(Content::deserialize_map_value(str, &mut map)?);
                        last_content = Some(str);
                    } else {
                        return Err(Error::unknown_field(str, Window::FIELDS));
                    }

                    if should_be_on_top && last_content.is_some() {
                        return Err(Error::custom(format!(
                            "all window properties should be above content, but `{}` is located after `{}`",
                            str, last_content.unwrap(),
                        )));
                    }
                }

                Ok(Window {
                    title: title.unwrap_or_default(),
                    props,
                    content,
                })
            }
        }

        deserializer.deserialize_struct("window", Self::FIELDS, TVisitor)
    }
}

//
// WindowProperty
//

#[derive(Debug, Clone)]
pub enum WindowProperty {
    Anchor(Anchor),
    TitleBar(bool),

    // everything related to resizing
    DefaultSize(egui::Vec2),
    MinSize(egui::Vec2),
    MaxSize(egui::Vec2),
    FixedSize(egui::Vec2),
    AutoSized,
    Resizable(bool),

    // other flags
    Enabled(bool),
    Interactable(bool),
    Movable(bool),
    Collapsible(bool),
}

impl WindowProperty {
    const FIELDS: &'static [&'static str] = &[
        "id", "anchor", "title_bar",
        "default_size", "min_size", "max_size", "fixed_size", "auto_sized", "resizable",
        "enabled", "interactable", "movable", "collapsible",
    ];

    fn deserialize_map_value<'de, A: MapAccess<'de>>(tag: &str, map: &mut A) -> Result<Self, A::Error> {
        match tag {
            "anchor"       => Ok(WindowProperty::Anchor       (map.next_value()?)),
            "title_bar"    => Ok(WindowProperty::TitleBar     (map.next_value()?)),
            "default_size" => Ok(WindowProperty::DefaultSize  (map.next_value::<Size<{ SIZE_ANY_DISALLOWED }>>()?.0)),
            "min_size"     => Ok(WindowProperty::MinSize      (map.next_value::<Size<{ SIZE_ANY_IS_ZERO    }>>()?.0)),
            "max_size"     => Ok(WindowProperty::MaxSize      (map.next_value::<Size<{ SIZE_ANY_IS_INF     }>>()?.0)),
            "fixed_size"   => Ok(WindowProperty::FixedSize    (map.next_value::<Size<{ SIZE_ANY_DISALLOWED }>>()?.0)),
            "auto_sized"   => { map.next_value::<Empty>()?;   Ok(WindowProperty::AutoSized) },
            "resizable"    => Ok(WindowProperty::Resizable    (map.next_value()?)),
            "enabled"      => Ok(WindowProperty::Enabled      (map.next_value()?)),
            "interactable" => Ok(WindowProperty::Interactable (map.next_value()?)),
            "movable"      => Ok(WindowProperty::Movable      (map.next_value()?)),
            "collapsible"  => Ok(WindowProperty::Collapsible  (map.next_value()?)),
            _              => Err(Error::unknown_field(tag, WindowProperty::FIELDS)),
        }
    }
}

//
// Content
//

#[derive(Debug, Clone)]
pub enum Content {
    Button(Button),
    Label(Label),
    Separator(Separator),
}

impl Content {
    const FIELDS: &'static [&'static str] = &["button", "label", "separator"];

    fn deserialize_map_value<'de, A: MapAccess<'de>>(tag: &str, map: &mut A) -> Result<Self, A::Error> {
        match tag {
            "button"    => Ok(Content::Button    (map.next_value()?)),
            "label"     => Ok(Content::Label     (map.next_value()?)),
            "separator" => Ok(Content::Separator (map.next_value()?)),
            _           => Err(Error::unknown_field(tag, Content::FIELDS)),
        }
    }
}

//
// Anchor
//

#[derive(Debug, Clone)]
pub struct Anchor {
    pub align: egui::Align2,
    pub offset: egui::Vec2,
}

impl<'de> Deserialize<'de> for Anchor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Anchor;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("{ align valign x y }")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut align_x = seq.next_element::<Alignment>()?.ok_or_else(|| Error::invalid_length(0, &self))?;
                let mut align_y = seq.next_element::<Alignment>()?.ok_or_else(|| Error::invalid_length(1, &self))?;

                if align_x.can_be_horizontal() && align_y.can_be_vertical() {
                    // all good
                } else if align_x.can_be_vertical() && align_y.can_be_horizontal() {
                    std::mem::swap(&mut align_x, &mut align_y);
                } else {
                    return Err(Error::custom(format!(
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

                let offset = if let Some(offset_x) = seq.next_element::<f32>()? {
                    let offset_y = seq.next_element::<f32>()?.ok_or_else(|| Error::invalid_length(3, &self))?;
                    if seq.next_element::<()>()?.is_some() {
                        return Err(Error::invalid_length(5, &self));
                    }
                    egui::Vec2::new(offset_x, offset_y)
                } else {
                    if seq.next_element::<()>()?.is_some() {
                        return Err(Error::invalid_length(3, &self));
                    }
                    egui::Vec2::ZERO
                };

                Ok(Anchor { align, offset })
            }
        }

        deserializer.deserialize_tuple_struct("anchor", 4, TVisitor)
    }
}

//
// RichText
//

#[derive(Default, Clone)]
pub struct RichText(pub egui::RichText);

impl RichText {
    const FIELDS: &'static [&'static str] = &["text"];
}

impl<'de> Deserialize<'de> for RichText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = egui::RichText;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("\"text\" or { text = .. }")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut text = None;
                let mut props = vec![];

                while let Some(str) = map.next_key::<&str>()? {
                    match str {
                        "text" => {
                            text = Some(map.next_value::<&str>()?);
                        }
                        str => {
                            props.push(RichTextProperty::deserialize_map_value(str, &mut map)?);
                        }
                    }
                }

                let text = text.ok_or_else(|| Error::missing_field("text"))?;
                let mut result = egui::RichText::new(text);

                for prop in props.iter() {
                    use RichTextProperty as P;
                    match prop {
                        P::Size(size) => {
                            result = result.size(*size);
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
                            result = result.color(*color);
                        }
                        P::BackgroundColor(color) => {
                            result = result.background_color(*color);
                        }
                        P::LineHeight(line_height) => {
                            result = result.line_height(Some(*line_height));
                        }
                        P::ExtraLetterSpacing(spacing) => {
                            result = result.extra_letter_spacing(*spacing);
                        }
                    }
                }

                Ok(result)
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(egui::RichText::new(value))
            }
        }

        Ok(RichText(deserializer.deserialize_struct("text", Self::FIELDS, TVisitor)?))
    }
}

impl Debug for RichText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: RichText doesn't implement debug in egui
        f.debug_struct("RichText")
            .field("text", &self.0.text())
            .finish()
    }
}

//
// RichTextProperty
//

#[derive(Debug, Clone)]
pub enum RichTextProperty {
    Size(f32),
    Style(Vec<RichTextStyle>),
    Color(egui::Color32),
    BackgroundColor(egui::Color32),
    LineHeight(f32),
    ExtraLetterSpacing(f32),
}

impl RichTextProperty {
    const FIELDS: &'static [&'static str] = &[
        "size", "style", "color", "background_color", "line_height", "extra_letter_spacing",
    ];

    fn deserialize_map_value<'de, A: MapAccess<'de>>(tag: &str, map: &mut A) -> Result<Self, A::Error> {
        match tag {
            "size"                 => Ok(RichTextProperty::Size               (map.next_value()?)),
            "extra_letter_spacing" => Ok(RichTextProperty::ExtraLetterSpacing (map.next_value()?)),
            "line_height"          => Ok(RichTextProperty::LineHeight         (map.next_value()?)),
            "style"                => Ok(RichTextProperty::Style              (map.next_value()?)),
            "background_color"     => Ok(RichTextProperty::BackgroundColor    (map.next_value::<Color>()?.0)),
            "color"                => Ok(RichTextProperty::Color              (map.next_value::<Color>()?.0)),
            _ => Err(Error::unknown_field(tag, RichTextProperty::FIELDS)),
        }
    }
}

//
// RichTextStyle
//

#[derive(serde::Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
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

//
// Button
//

#[derive(Debug, Clone)]
pub struct Button {
    pub name: Option<Name>,
    pub text: RichText,
    pub small: bool,
    pub props: Vec<ButtonProperty>,
}

impl Button {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["name", "text", "small"],
        ButtonProperty::FIELDS,
    );
}

impl<'de> Deserialize<'de> for Button {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Button;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("\"text\" or { text = .. }")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut name = None;
                let mut text = None;
                let mut small = false;
                let mut props = vec![];

                while let Some(str) = map.next_key::<&str>()? {
                    match str {
                        "name" => {
                            if name.is_some() { return Err(Error::duplicate_field("name")); }
                            name = Some(map.next_value()?);
                        }
                        "text" => {
                            if text.is_some() { return Err(Error::duplicate_field("text")); }
                            text = Some(map.next_value()?);
                        }
                        "small" => {
                            small = map.next_value()?;
                        }
                        str => {
                            props.push(ButtonProperty::deserialize_map_value(str, &mut map)?);
                        }
                    }
                }

                let text = text.ok_or_else(|| Error::missing_field("text"))?;

                Ok(Button { name, text, small, props })
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(Button {
                    name: None,
                    text: RichText(egui::RichText::new(value)),
                    small: false,
                    props: vec![],
                })
            }
        }

        deserializer.deserialize_struct("button", Self::FIELDS, TVisitor)
    }
}

//
// ButtonProperty
//

#[derive(Debug, Clone)]
pub enum ButtonProperty {
    ShortcutText(RichText),
    Wrap(bool),
    Fill(egui::Color32),
    Stroke(egui::Stroke),
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

    fn deserialize_map_value<'de, A: MapAccess<'de>>(tag: &str, map: &mut A) -> Result<Self, A::Error> {
        match tag {
            "shortcut_text" => Ok(ButtonProperty::ShortcutText (map.next_value()?)),
            "wrap"          => Ok(ButtonProperty::Wrap         (map.next_value()?)),
            "fill"          => Ok(ButtonProperty::Fill         (map.next_value::<Color>()?.0)),
            "stroke"        => Ok(ButtonProperty::Stroke       (map.next_value::<Stroke>()?.0)),
            "sense"         => Ok(ButtonProperty::Sense        (map.next_value()?)),
            "frame"         => Ok(ButtonProperty::Frame        (map.next_value()?)),
            "min_size"      => Ok(ButtonProperty::MinSize      (map.next_value::<Size<{ SIZE_ANY_IS_ZERO }>>()?.0)),
            "rounding"      => Ok(ButtonProperty::Rounding     (map.next_value::<Rounding>()?.0)),
            "selected"      => Ok(ButtonProperty::Selected     (map.next_value()?)),
            _               => Err(Error::unknown_field(tag, ButtonProperty::FIELDS)),
        }
    }
}

//
// Label
//

#[derive(Debug, Clone)]
pub struct Label {
    pub name: Option<Name>,
    pub text: RichText,
    pub props: Vec<LabelProperty>,
}

impl Label {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["name", "text"],
        LabelProperty::FIELDS,
    );
}

impl<'de> Deserialize<'de> for Label {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Label;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("\"text\" or { text = .. }")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut name = None;
                let mut text = None;
                let mut props = vec![];

                while let Some(str) = map.next_key::<&str>()? {
                    match str {
                        "name" => {
                            if name.is_some() { return Err(Error::duplicate_field("name")); }
                            name = Some(map.next_value()?);
                        }
                        "text" => {
                            if text.is_some() { return Err(Error::duplicate_field("text")); }
                            text = Some(map.next_value()?);
                        }
                        str => {
                            props.push(LabelProperty::deserialize_map_value(str, &mut map)?);
                        }
                    }
                }

                let text = text.ok_or_else(|| Error::missing_field("text"))?;

                Ok(Label { name, text, props })
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(Label {
                    name: None,
                    text: RichText(egui::RichText::new(value)),
                    props: vec![],
                })
            }
        }

        deserializer.deserialize_struct("label", Self::FIELDS, TVisitor)
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

    fn deserialize_map_value<'de, A: MapAccess<'de>>(tag: &str, map: &mut A) -> Result<Self, A::Error> {
        match tag {
            "wrap"     => Ok(LabelProperty::Wrap     (map.next_value()?)),
            "truncate" => Ok(LabelProperty::Truncate (map.next_value()?)),
            "sense"    => Ok(LabelProperty::Sense    (map.next_value()?)),
            _          => Err(Error::unknown_field(tag, WindowProperty::FIELDS)),
        }
    }
}

//
// Separator
//

#[derive(Debug, Clone)]
pub struct Separator {
    pub name: Option<Name>,
    pub is_horizontal: Option<bool>,
    pub props: Vec<SeparatorProperty>,
}

impl Separator {
    const FIELDS: &'static [&'static str] = const_concat!(
        &["name", "horizontal", "vertical"],
        SeparatorProperty::FIELDS,
    );
}

impl<'de> Deserialize<'de> for Separator {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Separator;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("{}")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut name = None;
                let mut is_horizontal = None;
                let mut props = vec![];

                while let Some(str) = map.next_key::<&str>()? {
                    match str {
                        "name" => {
                            if name.is_some() { return Err(Error::duplicate_field("name")); }
                            name = Some(map.next_value()?);
                        }
                        "horizontal" => { is_horizontal = Some(map.next_value::<bool>()?); }
                        "vertical"   => { is_horizontal = Some(!map.next_value::<bool>()?); }
                        str => { props.push(SeparatorProperty::deserialize_map_value(str, &mut map)?); }
                    }
                }

                Ok(Separator { name, is_horizontal, props })
            }
        }

        deserializer.deserialize_struct("separator", Self::FIELDS, TVisitor)
    }
}

//
// SeparatorProperty
//

#[derive(Debug, Clone)]
pub enum SeparatorProperty {
    Spacing(f32),
    Grow(f32),
    Shrink(f32),
}

impl SeparatorProperty {
    const FIELDS: &'static [&'static str] = &["spacing", "grow", "shrink"];

    fn deserialize_map_value<'de, A: MapAccess<'de>>(
        tag: &str,
        map: &mut A,
    ) -> Result<Self, A::Error> {
        match tag {
            "spacing"    => Ok(SeparatorProperty::Spacing    (map.next_value()?)),
            "grow"       => Ok(SeparatorProperty::Grow       (map.next_value()?)),
            "shrink"     => Ok(SeparatorProperty::Shrink     (map.next_value()?)),
            _            => Err(Error::unknown_field(tag, SeparatorProperty::FIELDS)),
        }
    }
}

//
// Name
//

#[derive(Debug, Clone)]
pub struct Name {
    pub id: Option<egui::Id>,
    pub str: String,
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Name;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("string")
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(Name { id: None, str: value.to_string() })
            }
        }

        deserializer.deserialize_str(TVisitor)
    }
}

//
// Alignment
//

#[derive(serde::Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
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

impl ToString for Alignment {
    fn to_string(&self) -> String {
        match self {
            Alignment::Center => "center",
            Alignment::Left   => "left",
            Alignment::Right  => "right",
            Alignment::Top    => "top",
            Alignment::Bottom => "bottom",
        }
        .to_string()
    }
}

//
// Color
//

#[derive(Debug, Clone)]
struct Color(egui::Color32);

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Color;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("{ r g b }")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                let value = serde_value::Value::String(v.into());
                let deserializer = serde_value::ValueDeserializer::new(value);
                let color_name = ColorName::deserialize(deserializer)?;
                Ok(Color(color_name.into()))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let r = seq.next_element::<u8>()?.ok_or_else(|| Error::invalid_length(0, &self))?;
                let g = seq.next_element::<u8>()?.ok_or_else(|| Error::invalid_length(1, &self))?;
                let b = seq.next_element::<u8>()?.ok_or_else(|| Error::invalid_length(2, &self))?;
                let a = seq.next_element::<u8>()?.unwrap_or(u8::MAX);
                if seq.next_element::<()>()?.is_some() {
                    return Err(Error::invalid_length(5, &self));
                }
                Ok(Color(egui::Color32::from_rgba_premultiplied(r, g, b, a)))
            }
        }

        deserializer.deserialize_any(TVisitor)
    }
}

//
// ColorName
//

#[derive(serde::Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
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

//
// Stroke
//

#[derive(Debug, Clone)]
pub struct Stroke(pub egui::Stroke);

impl<'de> Deserialize<'de> for Stroke {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Stroke;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("{ width color } or none")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let width = seq.next_element::<f32>()?.ok_or_else(|| Error::invalid_length(0, &self))?;
                let color = seq.next_element::<Color>()?.ok_or_else(|| Error::invalid_length(1, &self))?;
                if seq.next_element::<()>()?.is_some() {
                    return Err(Error::invalid_length(3, &self));
                }
                Ok(Stroke(egui::Stroke::new(width, color.0)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut width = None;
                let mut color = None;

                while let Some(str) = map.next_key::<&str>()? {
                    match str {
                        "width" => {
                            if width.is_some() { return Err(Error::duplicate_field("width")); }
                            width = Some(map.next_value::<f32>()?);
                        }
                        "color" => {
                            if color.is_some() { return Err(Error::duplicate_field("color")); }
                            color = Some(map.next_value::<Color>()?);
                        }
                        _ => { return Err(Error::unknown_field(str, &["width", "color"])); }
                    }
                }

                let width = width.ok_or_else(|| Error::missing_field("width"))?;
                let color = color.ok_or_else(|| Error::missing_field("color"))?;
                Ok(Stroke(egui::Stroke::new(width, color.0)))
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                if v == "none" {
                    Ok(Stroke(egui::Stroke::NONE))
                } else {
                    Err(Error::invalid_value(Unexpected::Str(v), &self))
                }
            }
        }

        deserializer.deserialize_any(TVisitor)
    }
}

//
// Rounding
//

#[derive(Debug, Clone)]
pub struct Rounding(pub egui::Rounding);

impl<'de> Deserialize<'de> for Rounding {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Rounding;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("none, number, or { top-left top-right bottom-right bottom-left }")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                // same semantics as in CSS
                let top_left     = seq.next_element::<f32>()?.ok_or_else(|| Error::invalid_length(0, &self))?;
                let top_right    = seq.next_element::<f32>()?.unwrap_or(top_left);
                let bottom_right = seq.next_element::<f32>()?.unwrap_or(top_left);
                let bottom_left  = seq.next_element::<f32>()?.unwrap_or(top_right);

                if seq.next_element::<()>()?.is_some() {
                    return Err(Error::invalid_length(5, &self));
                }

                Ok(Rounding(egui::Rounding {
                    nw: top_left,
                    ne: top_right,
                    se: bottom_right,
                    sw: bottom_left,
                }))
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                if v == "none" {
                    Ok(Rounding(egui::Rounding::ZERO))
                } else {
                    let rounding: f32 = v.parse()
                        .map_err(|_| Error::invalid_value(Unexpected::Str(v), &self))?;
                    Ok(Rounding(egui::Rounding::same(rounding)))
                }
            }
        }

        deserializer.deserialize_any(TVisitor)
    }
}

//
// Sense
//

#[derive(Debug, Clone)]
pub struct Sense(pub egui::Sense);

impl<'de> Deserialize<'de> for Sense {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Sense;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("{ click drag focusable }")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                #[derive(Deserialize)]
                #[serde(rename_all = "snake_case")]
                enum SenseKind {
                    Hover,
                    FocusableNoninteractive,
                    Click,
                    Drag,
                    ClickAndDrag,
                }

                let value = serde_value::Value::String(v.into());
                let deserializer = serde_value::ValueDeserializer::new(value);
                let sense_kind = SenseKind::deserialize(deserializer)?;
                let sense = match sense_kind {
                    SenseKind::Hover                   => egui::Sense::hover(),
                    SenseKind::FocusableNoninteractive => egui::Sense::focusable_noninteractive(),
                    SenseKind::Click                   => egui::Sense::click(),
                    SenseKind::Drag                    => egui::Sense::drag(),
                    SenseKind::ClickAndDrag            => egui::Sense::click_and_drag(),
                };
                Ok(Sense(sense))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                #[derive(Deserialize)]
                enum SenseType {
                    Click,
                    Drag,
                    Focusable,
                }

                let mut sense = egui::Sense::hover();

                while let Some(sense_type) = seq.next_element::<SenseType>()? {
                    match sense_type {
                        SenseType::Click     => sense.click = true,
                        SenseType::Drag      => sense.drag = true,
                        SenseType::Focusable => sense.focusable = true,
                    }
                }

                Ok(Sense(sense))
            }
        }

        deserializer.deserialize_any(TVisitor)
    }
}

//
// Size
//

const SIZE_ANY_IS_ZERO: u8 = 0;
const SIZE_ANY_IS_INF: u8 = 1;
const SIZE_ANY_DISALLOWED: u8 = 2;
struct Size<const ANY: u8>(egui::Vec2);

impl<'de, const ANY: u8> Deserialize<'de> for Size<ANY> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor<const ANY: u8>;

        impl<'de, const ANY: u8> Visitor<'de> for TVisitor<ANY> {
            type Value = Size<ANY>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("{ x y }")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                if ANY == SIZE_ANY_DISALLOWED {
                    let x = seq.next_element::<f32>()?.ok_or_else(|| Error::invalid_length(0, &self))?;
                    let y = seq.next_element::<f32>()?.ok_or_else(|| Error::invalid_length(1, &self))?;
                    if seq.next_element::<()>()?.is_some() {
                        return Err(Error::invalid_length(3, &self));
                    }
                    Ok(Size(egui::Vec2::new(x, y)))
                } else {
                    let x = seq.next_element::<AnyOrF32>()?.ok_or_else(|| Error::invalid_length(0, &self))?.0;
                    let y = seq.next_element::<AnyOrF32>()?.ok_or_else(|| Error::invalid_length(1, &self))?.0;
                    if seq.next_element::<()>()?.is_some() {
                        return Err(Error::invalid_length(3, &self));
                    }
                    Ok(Size(egui::Vec2::new(
                        x.unwrap_or(if ANY == SIZE_ANY_IS_ZERO { 0.0 } else { f32::INFINITY }),
                        y.unwrap_or(if ANY == SIZE_ANY_IS_ZERO { 0.0 } else { f32::INFINITY }),
                    )))
                }
            }
        }

        deserializer.deserialize_tuple_struct("size", 2, TVisitor::<ANY>)
    }
}

//
// AnyOrF32
//

struct AnyOrF32(Option<f32>);

impl<'de> Deserialize<'de> for AnyOrF32 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = AnyOrF32;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("number or `any`")
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                if value == "any" {
                    Ok(AnyOrF32(None))
                } else {
                    Err(Error::invalid_value(Unexpected::Str(value), &self))
                }
            }

            fn visit_f32<E: Error>(self, value: f32) -> Result<Self::Value, E> {
                Ok(AnyOrF32(Some(value)))
            }

            fn visit_f64<E: Error>(self, value: f64) -> Result<Self::Value, E> {
                self.visit_f32(value as f32)
            }
        }

        deserializer.deserialize_f32(TVisitor)
    }
}

//
// Empty
//

// This struct only allows `{}` and nothing else.
struct Empty;

impl<'de> Deserialize<'de> for Empty {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Empty;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("{}")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                if let Some(_next) = seq.next_element::<()>()? {
                    return Err(Error::invalid_type(Unexpected::Seq, &self));
                }

                Ok(Empty)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                if let Some(_next) = map.next_key::<()>()? {
                    return Err(Error::invalid_type(Unexpected::Map, &self));
                }

                Ok(Empty)
            }
        }

        deserializer.deserialize_any(TVisitor)
    }
}
