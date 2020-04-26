use crate::param::{ParamKind, ParamList, ParamStruct};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::events::attributes::Attributes;
use quick_xml::{Reader, Writer};

use std::io::{BufRead, Read, Cursor, Write, Error as ioError};
use std::str::{from_utf8, FromStr, Utf8Error};

/// Write a ParamStruct as XML
pub fn write_xml<W: Write>(param: &ParamStruct, writer: &mut W) -> Result<(), quick_xml::Error> {
    let mut xml_writer = Writer::new_with_indent(writer, b' ', 2);
    xml_writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"utf-8"), None)))?;
    struct_to_node(param, &mut xml_writer, None)?;
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

/// Read a ParamStruct from XML
pub fn read_xml<R: BufRead>(buf_reader: &mut R) -> Result<ParamStruct, ReadError> {
    let mut reader = Reader::from_reader(buf_reader);
    reader.expand_empty_elements(true);
    reader.trim_text(true);
    let mut buf = Vec::with_capacity(0x100);
    let mut stack = ParamStack::with_capacity(0x100);

    let res = read_xml_loop(&mut reader, &mut buf, &mut stack);

    if res.is_err() {
        println!("Error occurred. Position in data stream: {}", reader.buffer_position())
    }

    res
}

#[derive(Debug)]
/// Types of errors encountered the XML param file
pub enum ReadError {
    /// `quick-xml` error, such as mismatched tags, non-utf8 text, broken syntax, etc
    QuickXml(quick_xml::Error),
    /// Value parsing error
    ParseError,
    /// Opening tag has an unknown name
    UnknownOpenTag(String),
    /// Close tag name doesn't match open tag name
    UnmatchedCloseTag(String),
    /// For child nodes of structs, the 'hash' attribute is not found
    MissingHash,
    /// For the first tag after XML declaration, tag must be 'struct'
    ExpectedStructTag,
    /// After reading a struct or list tag, reader either expects a new open tag
    /// or that param's own close tag
    ExpectedOpenOrCloseTag(String),
    /// For after reading a value-type param and its value, expects the close tag
    ExpectedCloseTag(String),
    /// After reading the open tag for a value-type param, expects the text value
    ExpectedText,
    /// Any XML event not handled
    UnhandledEvent(QuickXmlEventType)
}

// impl ReadError {
//     pub fn print_location<R: Read>(&self, input: &R, position: usize) -> Result<(), ioError> {
//         // get line number
//         let mut line = 1;
//         // print line and show position by using a ^ caret sign
//         let reader = Cursor::new(input);
    
//         Ok(())
//     }
// }

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

impl<'a> From<&'a Event<'a>> for QuickXmlEventType {
    fn from(f: &Event) -> Self {
        match f {
            Event::CData(_) => Self::CData,
            Event::Comment(_) => Self::Comment,
            Event::Decl(_) => Self::Decl,
            Event::DocType(_) => Self::DocType,
            Event::Empty(_) => Self::Empty,
            Event::End(_) => Self::End,
            Event::Eof => Self::Eof,
            Event::PI(_) => Self::PI,
            Event::Start(_) => Self::Start,
            Event::Text(_) => Self::Text,
        }
    }
}

impl From<quick_xml::Error> for ReadError {
    fn from(f: quick_xml::Error) -> Self {
        Self::QuickXml(f)
    }
}

impl From<Utf8Error> for ReadError {
    fn from(f: Utf8Error) -> Self {
        Self::QuickXml(quick_xml::Error::from(f))
    }
}

impl From<ioError> for ReadError {
    fn from(f: ioError) -> Self {
        Self::QuickXml(quick_xml::Error::from(f))
    }
}

impl<'a> From<&'a Expect<'a>> for ReadError {
    fn from(f: &Expect) -> Self {
        match f {
            Expect::Struct => Self::ExpectedStructTag,
            Expect::OpenOrCloseTag(value) => {
                match from_utf8(value) {
                    Ok(s) => Self::ExpectedOpenOrCloseTag(String::from(s)),
                    Err(e) => Self::from(e)
                }
            },
            Expect::CloseTag(value) => {
                match from_utf8(value) {
                    Ok(s) => Self::ExpectedCloseTag(String::from(s)),
                    Err(e) => Self::from(e)
                }
            }
            Expect::Text => Self::ExpectedText,
        }
    }
}

#[derive(Debug)]
struct ParamStack<'a> {
    pub stack: Vec<ParamKind>,
    pub expect: Expect<'a>,
}

impl<'a> ParamStack<'a> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            stack: Vec::with_capacity(capacity),
            expect: Expect::Struct,
        }
    }

    fn last_mut(&mut self) -> &mut ParamKind {
        self.stack.last_mut().unwrap()
    }

    fn push(&mut self, node_name: &[u8], attributes: Attributes) -> Result<(), ReadError> {
        match self.expect {
            Expect::Struct => {
                if node_name == b"struct" {
                    self.expect = Expect::OpenOrCloseTag(b"struct");
                    self.stack.push(ParamKind::Struct(Default::default()));
                    Ok(())
                } else {
                    Err(ReadError::ExpectedStructTag)
                }
            }
            Expect::OpenOrCloseTag(_) => {
                macro_rules! default {
                    ($p:path) => {{
                        self.expect = Expect::Text;
                        $p(Default::default())
                    };
                }}
                let p = match node_name {
                    b"bool" => default!(ParamKind::Bool),
                    b"sbyte" => default!(ParamKind::I8),
                    b"byte" => default!(ParamKind::U8),
                    b"short" => default!(ParamKind::I16),
                    b"ushort" => default!(ParamKind::U16),
                    b"int" => default!(ParamKind::I32),
                    b"uint" => default!(ParamKind::U32),
                    b"float" => default!(ParamKind::Float),
                    b"hash40" => default!(ParamKind::Hash),
                    b"string" => default!(ParamKind::Str),
                    b"list" => {
                        self.expect = Expect::OpenOrCloseTag(b"list");
                        ParamKind::List(Default::default())
                    }
                    b"struct" => {
                        self.expect = Expect::OpenOrCloseTag(b"struct");
                        ParamKind::Struct(Default::default())
                    }
                    _ => return Err(ReadError::UnknownOpenTag(
                        String::from(from_utf8(node_name)?))
                    ),
                };

                if let ParamKind::Struct(s) = self.last_mut() {
                    let hash = attributes
                        .collect::<Result<Vec<_>, _>>()?
                        .iter()
                        .find(|attr| attr.key == b"hash")
                        .ok_or(ReadError::MissingHash)
                        .and_then(|attr| {
                            FromStr::from_str(from_utf8(&attr.value)?)
                                .or(Err(ReadError::MissingHash))
                        })?;
                    // push a temporary param into the struct with the real hash
                    // because we don't have a way to store this for later, when
                    // the close tag is reached (unless I make something for it)
                    s.push((hash, ParamKind::Bool(Default::default())));
                }

                self.stack.push(p);
                Ok(())
            }
            Expect::CloseTag(name) => {
                Err(ReadError::ExpectedCloseTag(String::from(from_utf8(name)?)))
            }
            Expect::Text => unreachable!(),
        }
    }

    fn pop(&mut self, node_name: &[u8]) -> Result<Option<ParamStruct>, ReadError> {
        match self.expect {
            Expect::CloseTag(name) | Expect::OpenOrCloseTag(name) => {
                if name != node_name {
                    return Err(ReadError::UnmatchedCloseTag(String::from(from_utf8(name)?)))
                }
                // take the param off the stack
                let p = self.stack.pop().unwrap();
                // insert it into the parent
                // by the way expect is set, the parent is guaranteed to be either a struct or list
                match self.stack.last_mut() {
                    Some(ParamKind::Struct(children)) => {
                        children.last_mut().unwrap().1 = p;
                        self.expect = Expect::OpenOrCloseTag(b"struct");
                    }
                    Some(ParamKind::List(children)) => {
                        children.push(p);
                        self.expect = Expect::OpenOrCloseTag(b"list");
                    }
                    None => {
                        // first param on stack is guaranteed to be a struct
                        // so when we pop and there's nothing left, 'p' is that struct
                        if let ParamKind::Struct(s) = p {
                            return Ok(Some(s))
                        } else {
                            unreachable!()
                        }
                    },
                    _ => unreachable!(),
                }
            }
            Expect::Struct => return Err(ReadError::ExpectedStructTag),
            // we can't expect a text event here because quick-xml always creates a text event between tags
            // which will be handled in another function and change expect respectively
            _ => unreachable!(),
        }

        Ok(None)
    }

    fn handle_text(&mut self, text: &[u8]) -> Result<(), ReadError> {
        if let Expect::Text = self.expect {
            let top = self.last_mut();
            macro_rules! convert {
                ($t:path, $tag_name:literal) => {{
                    *top = FromStr::from_str(from_utf8(text)?)
                        .map($t)
                        .or(Err(ReadError::ParseError))?;
                    self.expect = Expect::CloseTag($tag_name);
                    Ok(())
                };
            }}

            match top {
                ParamKind::Bool(_) => convert!(ParamKind::Bool, b"bool"),
                ParamKind::I8(_) => convert!(ParamKind::I8, b"sbyte"),
                ParamKind::U8(_) => convert!(ParamKind::U8, b"byte"),
                ParamKind::I16(_) => convert!(ParamKind::I16, b"short"),
                ParamKind::U16(_) => convert!(ParamKind::U16, b"ushort"),
                ParamKind::I32(_) => convert!(ParamKind::I32, b"int"),
                ParamKind::U32(_) => convert!(ParamKind::U32, b"uint"),
                ParamKind::Float(_) => convert!(ParamKind::Float, b"float"),
                ParamKind::Hash(_) => convert!(ParamKind::Hash, b"hash40"),
                ParamKind::Str(_) => convert!(ParamKind::Str, b"string"),
                // Note for readers
                // Expect is only set to Text after reading a value-type open tag
                // The two cases below are designed to be impossible
                ParamKind::List(_) => unreachable!(),
                ParamKind::Struct(_) => unreachable!(),
            }
        } else if text.is_empty() {
            // empty text event being sent from quick-xml is meaningless
            Ok(())
        } else {
            Err(ReadError::from(&self.expect))
        }
    }
}

/// XML Reading state handling
#[derive(Debug, Clone)]
enum Expect<'a> {
    /// Should only be used at the start of the file
    Struct,
    /// After reading a list or struct, expects either the close tag
    /// Or any open tag for a new param
    OpenOrCloseTag(&'a [u8]),
    /// After parsing a text event out of a value-type param, expects this close tag.
    /// Instead of a stack of strings, this gets set when the stack is changed
    CloseTag(&'a [u8]),
    /// Used for the inside of value-type params
    Text,
}

fn read_xml_loop<R: BufRead>(reader: &mut Reader<R>, buf: &mut Vec<u8>, stack: &mut ParamStack) -> Result<ParamStruct, ReadError> {
    loop {
        let event = reader.read_event(buf)?;
        match event {
            Event::Start(start) => stack.push(start.name(), start.attributes())?,
            Event::Text(text) => stack.handle_text(&*text)?,
            Event::End(end) => {
                if let Some(p) = stack.pop(end.name())? {
                    return Ok(p)
                }
            }
            Event::Decl(_) => {}
            _ => return Err(ReadError::UnhandledEvent(QuickXmlEventType::from(&event))),
        }

        buf.clear();
    }
}
