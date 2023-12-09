use std::fmt::Debug;

use serde::de::{Error, MapAccess, SeqAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};

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
    Id(egui::Id),
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
            "id"           => Ok(WindowProperty::Id           (egui::Id::new(map.next_value::<&str>()?))),
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
    Label(Label),
    Separator,
}

impl Content {
    const FIELDS: &'static [&'static str] = &["label", "separator"];

    fn deserialize_map_value<'de, A: MapAccess<'de>>(tag: &str, map: &mut A) -> Result<Self, A::Error> {
        match tag {
            "label" => Ok(Content::Label(map.next_value()?)),
            "separator" => {
                map.next_value::<Empty>()?;
                Ok(Content::Separator)
            },
            _ => Err(Error::unknown_field(tag, Content::FIELDS)),
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
                        Alignment::Left => egui::Align::Min,
                        Alignment::Center => egui::Align::Center,
                        Alignment::Right => egui::Align::Max,
                        _ => unreachable!(),
                    },
                    match align_y {
                        Alignment::Top => egui::Align::Min,
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
// Label
//

#[derive(Debug, Clone)]
pub struct Label {
    pub text: String,
}

impl Label {
    const FIELDS: &'static [&'static str] = &["text"];
}

impl<'de> Deserialize<'de> for Label {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TVisitor;

        impl<'de> Visitor<'de> for TVisitor {
            type Value = Label;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("\"text\" or { text = .. }")
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut text = None;

                while let Some(str) = map.next_key::<&str>()? {
                    match str {
                        "text" => {
                            if text.is_some() { return Err(Error::duplicate_field("text")); }
                            text = Some(map.next_value()?);
                        }
                        _ => { return Err(Error::unknown_field(str, Label::FIELDS)); }
                    }
                }

                let text = text.ok_or_else(|| Error::missing_field("text"))?;

                Ok(Label { text })
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(Label { text: value.to_string() })
            }
        }

        deserializer.deserialize_struct("label", Self::FIELDS, TVisitor)
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
            Alignment::Left => "left",
            Alignment::Right => "right",
            Alignment::Top => "top",
            Alignment::Bottom => "bottom",
        }.to_string()
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
