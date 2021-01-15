use crate::param::{ParamKind, ParamList, ParamStruct};
use quick_xml::events::attributes::Attributes;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use std::io::{BufRead, Error as ioError, Read, Write};
use std::str::{from_utf8, FromStr, Utf8Error};

pub use quick_xml;

/// Writes a ParamStruct as XML into the given writer.
/// Returns nothing if successful, otherwise an [quick_xml::Error](Error).
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

    if param.0.is_empty() {
        writer.write_event(Event::Empty(start))?;
    } else {
        writer.write_event(Event::Start(start))?;
        for (index, child) in param.0.iter().enumerate() {
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

    if param.0.is_empty() {
        writer.write_event(Event::Empty(start))?;
    } else {
        writer.write_event(Event::Start(start))?;
        for (hash, child) in param.0.iter() {
            param_to_node(child, writer, Some(("hash", &format!("{}", hash))))?;
        }
        writer.write_event(Event::End(BytesEnd::borrowed(name)))?;
    }
    Ok(())
}

/// Read a ParamStruct from the given reader over XML data.
/// Returns the param if successful, otherwise a [ReadErrorWrapper].
pub fn read_xml<R: BufRead>(buf_reader: &mut R) -> Result<ParamStruct, ReadErrorWrapper> {
    let mut reader = Reader::from_reader(buf_reader);
    reader.expand_empty_elements(true);
    reader.trim_text(true);
    let mut buf = Vec::with_capacity(0x100);
    let mut stack = ParamStack::with_capacity(0x100);

    read_xml_loop(&mut reader, &mut buf, &mut stack)
}

/// Takes a reader into the source file, the start and end position of any error, and returns an error string.
/// The error string represents the file contents for all lines in the error range,
/// as well as start and end pointers indicating the error's exact location.
/// Returns an [Error](ioError) if the reader fails in the process.
/// Panics if the start value is greater than the end value, or if the stream is too short for either.
pub fn get_xml_error<R: Read>(reader: R, start: usize, end: usize) -> Result<String, ioError> {
    if start > end {
        panic!(
            "The provided start position ({}) must be less than or equal to the provided end position ({})",
            start, end
        )
    }

    let mut ret = String::default();

    #[derive(PartialEq)]
    enum Stage {
        One,
        Two,
        Three,
    }
    let mut stage = Stage::One;
    let mut line_so_far = Vec::with_capacity(0x40);
    let mut line_start = 0;
    let mut line_num = 1;

    for (position, byte_res) in reader.bytes().enumerate() {
        let byte = byte_res?;
        match stage {
            Stage::One => {
                if position >= start {
                    stage = Stage::Two;
                    line_so_far.push(byte);
                } else if byte == b'\n' {
                    line_so_far.clear();
                    line_start = position;
                    line_num += 1;
                } else {
                    line_so_far.push(byte);
                }
            }
            Stage::Two => {
                line_so_far.push(byte);
                if position >= end {
                    stage = Stage::Three;
                }
            }
            Stage::Three => {
                if byte == b'\n' {
                    break;
                } else {
                    line_so_far.push(byte);
                }
            }
        }
    }
    if stage != Stage::Three {
        panic!("The provided start or end values were longer than the provided stream")
    }

    let first = String::from_utf8_lossy(&line_so_far[0..start - line_start]);
    let middle = String::from_utf8_lossy(&line_so_far[start - line_start..end - line_start]);
    let last = String::from_utf8_lossy(&line_so_far[end - line_start..]);

    let combined = format!("{}{}{}", first, middle, last);
    let max_line_num = line_num + combined.lines().count() - 1;
    let line_count_length = format!("{}", max_line_num).len();
    let mut last_line = String::default();

    ret.push_str(format!("{}v\n", " ".repeat(line_count_length + 1 + first.len())).as_ref());
    for (index, line) in combined.lines().enumerate() {
        last_line = format!(
            "{:len$}: {}",
            index + line_num,
            line,
            len = line_count_length
        );
        ret.push_str(format!("{}\n", last_line).as_ref());
    }
    ret.push_str(format!("{}^\n", " ".repeat(last_line.len() - last.len() - 1)).as_ref());

    Ok(ret)
}

/// A wrapper over the error returned from reading XML.
/// Provides start and end positions useful for [get_xml_error].
#[derive(Debug)]
pub struct ReadErrorWrapper {
    pub error: ReadError,
    pub start: usize,
    pub end: usize,
}

impl ReadErrorWrapper {
    pub fn new(error: ReadError, start: usize, end: usize) -> Self {
        Self { error, start, end }
    }
}

/// Types of errors encountered while reading the XML param file
#[derive(Debug)]
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
    UnhandledEvent(QuickXmlEventType),
}

// Bad practice to just copy event names?
// I need to have an "expected" event type as well so I can't just use Event<'a>
/// A bare enum recording possible XML events, named to mirror [Event]
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
            Expect::OpenOrCloseTag(value) => match from_utf8(value) {
                Ok(s) => Self::ExpectedOpenOrCloseTag(String::from(s)),
                Err(e) => Self::from(e),
            },
            Expect::CloseTag(value) => match from_utf8(value) {
                Ok(s) => Self::ExpectedCloseTag(String::from(s)),
                Err(e) => Self::from(e),
            },
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
                    };};
                }
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
                    _ => {
                        return Err(ReadError::UnknownOpenTag(String::from(from_utf8(
                            node_name,
                        )?)))
                    }
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
                    s.0.push((hash, ParamKind::Bool(Default::default())));
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
                    return Err(ReadError::UnmatchedCloseTag(String::from(from_utf8(name)?)));
                }
                // take the param off the stack
                let p = self.stack.pop().unwrap();
                // insert it into the parent
                // by the way expect is set, the parent is guaranteed to be either a struct or list
                match self.stack.last_mut() {
                    Some(ParamKind::Struct(children)) => {
                        children.0.last_mut().unwrap().1 = p;
                        self.expect = Expect::OpenOrCloseTag(b"struct");
                    }
                    Some(ParamKind::List(children)) => {
                        children.0.push(p);
                        self.expect = Expect::OpenOrCloseTag(b"list");
                    }
                    None => {
                        // first param on stack is guaranteed to be a struct
                        // so when we pop and there's nothing left, 'p' is that struct
                        return Ok(Some(p.try_into_owned().unwrap()));
                    }
                    _ => unreachable!(),
                }
            }
            Expect::Struct => return Err(ReadError::ExpectedStructTag),
            // if we get a close tag but no text, default behavior is to let
            // the param keeps its default value. Then we just
            Expect::Text => {
                self.handle_text(b"")?;
                self.pop(node_name)?;
            }
        }

        Ok(None)
    }

    fn handle_text(&mut self, text: &[u8]) -> Result<(), ReadError> {
        if let Expect::Text = self.expect {
            let top = self.last_mut();
            macro_rules! convert {
                ($t:path, $tag_name:literal) => {{
                    if !text.is_empty() {
                        *top = FromStr::from_str(from_utf8(text)?)
                            .map($t)
                            .or(Err(ReadError::ParseError))?;
                    }
                    self.expect = Expect::CloseTag($tag_name);
                    Ok(())
                };};
            }

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

fn read_xml_loop<R: BufRead>(
    reader: &mut Reader<R>,
    buf: &mut Vec<u8>,
    stack: &mut ParamStack,
) -> Result<ParamStruct, ReadErrorWrapper> {
    let mut pre_position;
    loop {
        pre_position = reader.buffer_position();
        macro_rules! try_with_position {
            ($run:expr) => {
                match $run {
                    Ok(ok) => ok,
                    Err(e) => {
                        return Err(ReadErrorWrapper::new(
                            ReadError::from(e),
                            pre_position,
                            reader.buffer_position() - 1,
                        ))
                    }
                }
            };
        }
        let event = try_with_position!(reader.read_event(buf));
        match event {
            Event::Start(start) => try_with_position!(stack.push(start.name(), start.attributes())),
            Event::Text(text) => try_with_position!(stack.handle_text(&*text)),
            Event::End(end) => {
                if let Some(p) = try_with_position!(stack.pop(end.name())) {
                    return Ok(p);
                }
            }
            Event::Decl(_) => {}
            _ => {
                return Err(ReadErrorWrapper::new(
                    ReadError::UnhandledEvent(QuickXmlEventType::from(&event)),
                    pre_position,
                    reader.buffer_position(),
                ))
            }
        }

        buf.clear();
    }
}
