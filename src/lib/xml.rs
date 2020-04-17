use crate::param::{ParamKind, ParamList, ParamStruct};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use std::io::{BufRead, Write};
use std::str::{from_utf8, FromStr, Utf8Error};

#[derive(Debug)]
pub struct ErrorPos {
    line: usize,
    column: usize,
}

#[derive(Debug)]
/// An error produced reading the XML param file
pub struct ReadError {
    /// The location of the reader in the file at the time of error
    pub pos: ErrorPos,
    /// The type of error
    pub variant: ReadErrorVariant,
}

#[derive(Debug)]
/// Types of errors encountered the XML param file
pub enum ReadErrorVariant {
    /// `quick-xml` error, such as mismatched tags, non-utf8 text, broken syntax, etc
    QuickXml(quick_xml::Error),
    /// Wrong event received
    /// Received / Expected
    UnexpectedEvent(QuickXmlEventType, QuickXmlEventType),
    /// Value parsing error
    ParseError,
    UnknownOpenTag(String),
    UnmatchedCloseTag(String),
    MissingHash
}

// Bad practice to just copy event names?
// I need to have an "expected" event type as well so I can't just use Event<'a>
#[derive(Debug, Clone, Copy)]
pub enum QuickXmlEventType {
    CData,
    Comment,
    Decl,
    DocType,
    Empty,
    End,
    Eof,
    PI,
    Start,
    Text,
}

impl From<quick_xml::Error> for ReadErrorVariant {
    fn from(f: quick_xml::Error) -> Self {
        Self::QuickXml(f)
    }
}

impl<'a> From<Event<'a>> for QuickXmlEventType {
    fn from(f: Event) -> Self {
        match f {
            Event::CData(..) => Self::CData,
            Event::Comment(..) => Self::Comment,
            Event::Decl(..) => Self::Decl,
            Event::DocType(..) => Self::DocType,
            Event::Empty(..) => Self::Empty,
            Event::End(..) => Self::End,
            Event::Eof => Self::Eof,
            Event::PI(..) => Self::PI,
            Event::Start(..) => Self::Start,
            Event::Text(..) => Self::Text,
        }
    }
}

impl From<Utf8Error> for ReadErrorVariant {
    fn from(f: Utf8Error) -> Self {
        Self::QuickXml(quick_xml::Error::from(f))
    }
}

struct MainReader<B: BufRead> {
    reader: Reader<B>,
    buf: Vec<u8>,
}

pub fn write_xml<W: Write>(param: &ParamStruct, writer: &mut W) -> Result<(), quick_xml::Error> {
    let mut xml_writer = Writer::new_with_indent(writer, b' ', 2);
    xml_writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"utf-8"), None)))?;
    struct_to_node(param, &mut xml_writer, None)?;
    Ok(())
}

pub fn read_xml<R: BufRead>(reader: &mut R) -> Result<ParamKind, ReadError> {
    let mut xml_reader = Reader::from_reader(reader).expand_empty_elements(true);
    let mut buf = Vec::<u8>::new();
    loop {}
}

fn list_to_node<W: Write>(
    param: &ParamList,
    writer: &mut Writer<W>,
    attr: Option<(&str, &str)>,
) -> Result<(), quick_xml::Error> {
    let name = b"list";
    let mut start = BytesStart::borrowed_name(name);
    if let Some(a) = attr {
        start.push_attribute(a);
    }

    if param.is_empty() {
        writer.write_event(Event::Empty(start))?;
    } else {
        writer.write_event(Event::Start(start))?;
        for (index, child) in param.iter().enumerate() {
            param_to_node(child, writer, Some(("index", &format!("{}", index))))?;
        }
        writer.write_event(Event::End(BytesEnd::borrowed(name)))?;
    }
    Ok(())
}

fn struct_to_node<W: Write>(
    param: &ParamStruct,
    writer: &mut Writer<W>,
    attr: Option<(&str, &str)>,
) -> Result<(), quick_xml::Error> {
    let name = b"struct";
    let mut start = BytesStart::borrowed_name(name);
    if let Some(a) = attr {
        start.push_attribute(a);
    }

    if param.is_empty() {
        writer.write_event(Event::Empty(start))?;
    } else {
        writer.write_event(Event::Start(start))?;
        for (hash, child) in param.iter() {
            param_to_node(child, writer, Some(("hash", &format!("{}", hash))))?;
        }
        writer.write_event(Event::End(BytesEnd::borrowed(name)))?;
    }
    Ok(())
}

fn param_to_node<W: Write>(
    param: &ParamKind,
    writer: &mut Writer<W>,
    attr: Option<(&str, &str)>,
) -> Result<(), quick_xml::Error> {
    macro_rules! write_constant {
        ($tag_name:literal, $value:expr) => {{
            let name = $tag_name;
            let mut start = BytesStart::borrowed_name(name);
            if let Some(a) = attr {
                start.push_attribute(a);
            }
            writer.write_event(Event::Start(start))?;
            // Is there any shorter way of expressing this?
            writer.write_event(Event::Text(BytesText::from_plain_str(&format!(
                "{}",
                $value
            ))))?;
            writer.write_event(Event::End(BytesEnd::borrowed(name)))?;
        }};
    };
    match param {
        ParamKind::Bool(val) => write_constant!(b"bool", val),
        ParamKind::I8(val) => write_constant!(b"sbyte", val),
        ParamKind::U8(val) => write_constant!(b"byte", val),
        ParamKind::I16(val) => write_constant!(b"short", val),
        ParamKind::U16(val) => write_constant!(b"ushort", val),
        ParamKind::I32(val) => write_constant!(b"int", val),
        ParamKind::U32(val) => write_constant!(b"uint", val),
        ParamKind::Float(val) => write_constant!(b"float", val),
        ParamKind::Hash(val) => write_constant!(b"hash40", val),
        ParamKind::Str(val) => write_constant!(b"string", val),
        ParamKind::List(val) => list_to_node(val, writer, attr)?,
        ParamKind::Struct(val) => struct_to_node(val, writer, attr)?,
    };

    Ok(())
}

fn node_to_param<R: BufRead>(
    reader: &mut Reader<R>,
    buf: &mut Vec<u8>,
    open_tag: &[u8],
) -> Result<ParamKind, ReadErrorVariant> {
    macro_rules! read_constant {
        ($param_kind:path) => {
            {
                let val = reader.read_event(buf)?;
                if let Event::Text(bytes) = val {
                    let p = FromStr::from_str(from_utf8(&bytes)?)
                        .map($param_kind)
                        .or(Err(ReadErrorVariant::ParseError))?;

                    let close_tag = reader.read_event(buf)?;
                    if open_tag == &*close_tag {
                        Ok(p)
                    } else {
                        Err(ReadErrorVariant::UnmatchedCloseTag(from_utf8(&*bytes)?.to_string())) 
                    }
                } else {
                    Err(ReadErrorVariant::UnexpectedEvent(
                        QuickXmlEventType::Text,
                        QuickXmlEventType::from(val),
                    ))
                }
            }
        };
    }

    match open_tag {
        b"bool" => read_constant!(ParamKind::Bool),
        b"sbyte" => read_constant!(ParamKind::I8),
        b"byte" => read_constant!(ParamKind::U8),
        b"short" => read_constant!(ParamKind::I16),
        b"ushort" => read_constant!(ParamKind::U16),
        b"int" => read_constant!(ParamKind::I32),
        b"uint" => read_constant!(ParamKind::U32),
        b"float" => read_constant!(ParamKind::Float),
        b"hash40" => read_constant!(ParamKind::Hash),
        b"string" => read_constant!(ParamKind::Str),
        b"list" => Ok(ParamKind::List(node_to_list(reader, buf)?)),
        b"struct" => Ok(ParamKind::Struct(node_to_struct(reader, buf)?)),
        _ => Err(ReadErrorVariant::UnknownOpenTag(from_utf8(open_tag)?.to_string())),
    }
}

fn node_to_list<'a, R: BufRead>(reader: &mut Reader<R>, buf: &'a mut Vec<u8>) -> Result<ParamList, ReadErrorVariant> {
    let mut param_list = ParamList::new();
    loop {
        let event = reader.read_event(buf)?;
        match event {
            Event::Start(bytes) => { param_list.push(node_to_param(reader, buf, bytes.name())?) }
            Event::End(bytes) => {
                return if &*bytes == b"list" {
                    Ok(param_list)
                } else {
                    Err(ReadErrorVariant::UnmatchedCloseTag(from_utf8(&*bytes)?.to_string())) 
                }
            },
            _ => {
                return Err(ReadErrorVariant::UnexpectedEvent(
                    QuickXmlEventType::Start,
                    QuickXmlEventType::from(event),
                ))
            }
        }
    }
}

fn node_to_struct<R: BufRead>(reader: &mut Reader<R>, buf: &mut Vec<u8>) -> Result<ParamStruct, ReadErrorVariant> {
    let mut param_struct = ParamStruct::new();
    loop {
        let event = reader.read_event(buf)?;
        match event {
            Event::Start(bytes) => {
                param_struct.push((
                    bytes.attributes()
                        .collect::<Result<Vec<_>,_>>()?
                        .iter()
                        .find(|attr| attr.key == b"hash")
                        .ok_or(ReadErrorVariant::MissingHash)
                        .and_then(|attr|
                            FromStr::from_str(from_utf8(&attr.value)?)
                                .or(Err(ReadErrorVariant::MissingHash))
                        )?,
                    node_to_param(reader, buf, bytes.name())?
                ));
            }
            Event::End(bytes) => {
                return if &*bytes == b"struct" {
                    Ok(param_struct)
                } else {
                    Err(ReadErrorVariant::UnmatchedCloseTag(from_utf8(&*bytes)?.to_string())) 
                }
            },
            _ => {
                return Err(ReadErrorVariant::UnexpectedEvent(
                    QuickXmlEventType::Start,
                    QuickXmlEventType::from(event),
                ))
            }
        }

        buf.clear();
    }
}
