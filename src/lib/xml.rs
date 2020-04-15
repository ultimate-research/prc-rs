use crate::param::{ParamKind, ParamStruct, ParamList};
use quick_xml::{Reader, Writer, Error};
use quick_xml::events::{
    Event,
    BytesDecl,
    BytesStart,
    BytesText,
    BytesEnd
};
use std::io::{Read, Write};

pub fn to_xml<W: Write>(param: &ParamStruct, writer: &mut W) -> Result<(), Error> {
    let mut xml_writer = Writer::new_with_indent(writer, b' ', 2);
    xml_writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"utf-8"), None)))?;
    struct_to_node(param, &mut xml_writer, None)?;
    Ok(())
}

fn list_to_node<W: Write>(param: &ParamList, writer: &mut Writer<W>, attr: Option<(&str, &str)>) -> Result<(), Error> {
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

fn struct_to_node<W: Write>(param: &ParamStruct, writer: &mut Writer<W>, attr: Option<(&str, &str)>) -> Result<(), Error> {
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

fn param_to_node<W: Write>(param: &ParamKind, writer: &mut Writer<W>, attr: Option<(&str, &str)>) -> Result<(), Error> {
    macro_rules! write_constant {
        ($tag_name:literal, $value:expr) => {{
            let name = $tag_name;
            let mut start = BytesStart::borrowed_name(name);
            if let Some(a) = attr {
                start.push_attribute(a);
            }
            writer.write_event(Event::Start(start))?;
            // Is there any shorter way of expressing this?
            writer.write_event(Event::Text(BytesText::from_plain_str(&format!("{}", $value))))?;
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
