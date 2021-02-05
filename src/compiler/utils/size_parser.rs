use crate::{qjs, Error, Map, Result, Set};
use nom::{
    bytes::complete::{is_not, tag},
    character::complete::{char, digit1, line_ending, space1},
    combinator::{all_consuming, map, map_res, opt},
    multi::separated_list0,
    sequence::{delimited, tuple},
    Err as IErr, IResult,
};
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
    str::FromStr,
};

#[derive(Debug, Clone, qjs::IntoJs)]
pub struct SizeInfo {
    pub size: u64,
    pub sections: Map<String, u64>,
    pub objects: Set<ObjectSizeInfo>,
}

impl FromStr for SizeInfo {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        Ok(all_consuming(Self::parse_sysv)(input)
            .map_err(|error| match error {
                IErr::Error(error) => error.input,
                IErr::Failure(error) => error.input,
                _ => unreachable!(),
            })
            .map_err(|input| format!("Error while parsing size info: `{}`", input))?
            .1)
    }
}

#[derive(Debug, Clone, qjs::IntoJs)]
pub struct ObjectSizeInfo {
    pub name: String,
    pub archive: Option<String>,
    pub size: u64,
    pub sections: Set<SectionSizeInfo>,
}

impl Borrow<str> for ObjectSizeInfo {
    fn borrow(&self) -> &str {
        &self.name
    }
}

impl Borrow<String> for ObjectSizeInfo {
    fn borrow(&self) -> &String {
        &self.name
    }
}

impl PartialEq for ObjectSizeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.archive == other.archive && self.name == other.name
    }
}

impl Eq for ObjectSizeInfo {}

impl Hash for ObjectSizeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.archive.hash(state);
        self.name.hash(state);
    }
}

#[derive(Debug, Clone, qjs::IntoJs)]
pub struct SectionSizeInfo {
    pub name: String,
    pub address: u64,
    pub size: u64,
}

impl PartialEq for SectionSizeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Borrow<str> for SectionSizeInfo {
    fn borrow(&self) -> &str {
        &self.name
    }
}

impl Borrow<String> for SectionSizeInfo {
    fn borrow(&self) -> &String {
        &self.name
    }
}

impl Eq for SectionSizeInfo {}

impl Hash for SectionSizeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl SizeInfo {
    fn parse_sysv(input: &str) -> IResult<&str, Self> {
        map(
            separated_list0(tag("\n\n"), ObjectSizeInfo::parse_sysv),
            |objects| {
                let mut size = 0u64;
                let mut sections = Map::<String, u64>::default();
                let objects = objects
                    .into_iter()
                    .map(|object| {
                        size += object.size;
                        for section in &object.sections {
                            let size = sections.entry(section.name.clone()).or_insert(0);
                            *size += section.size;
                        }
                        object
                    })
                    .collect();
                Self {
                    size,
                    sections,
                    objects,
                }
            },
        )(input)
    }
}

impl ObjectSizeInfo {
    fn parse_sysv(input: &str) -> IResult<&str, Self> {
        map(
            tuple((
                Self::parse_sysv_head,
                line_ending,
                tuple((tag("section"), space1, tag("size"), space1, tag("addr"))),
                line_ending,
                separated_list0(line_ending, SectionSizeInfo::parse_sysv),
                line_ending,
                Self::parse_sysv_size,
                line_ending,
            )),
            |((name, archive), _, _, _, sections, _, size, _)| {
                let sections = sections.into_iter().collect();
                Self {
                    name,
                    archive,
                    size,
                    sections,
                }
            },
        )(input)
    }

    fn parse_sysv_head(input: &str) -> IResult<&str, (String, Option<String>)> {
        map(
            tuple((
                is_not(":("),
                opt(delimited(tag("(ex"), is_not(")"), char(')'))),
                char(':'),
            )),
            |(name, archive, _): (&str, Option<&str>, _)| {
                (name.trim().into(), archive.map(|name| name.trim().into()))
            },
        )(input)
    }

    fn parse_sysv_size(input: &str) -> IResult<&str, u64> {
        map(tuple((tag("Total"), space1, parse_size)), |(_, _, size)| {
            size
        })(input)
    }
}

impl SectionSizeInfo {
    fn parse_sysv(input: &str) -> IResult<&str, Self> {
        map(
            tuple((is_not(" \t"), space1, parse_size, space1, parse_size)),
            |(name, _, size, _, address): (&str, _, _, _, _)| Self {
                name: name.into(),
                address,
                size,
            },
        )(input)
    }
}

fn parse_size(input: &str) -> IResult<&str, u64> {
    map_res(digit1, u64::from_str)(input)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn single_object() {
        let input = r#"objs/hello.c.o  :
section           size   addr
.text               22      0
.data                0      0
.bss                 0      0
.rodata.str1.1      12      0
.comment            18      0
.note.GNU-stack      0      0
.eh_frame           48      0
Total              100
"#;
        let info: SizeInfo = input.parse().unwrap();
        assert_eq!(info.objects.len(), 1);
        assert_eq!(info.objects[0].name, "objs/hello.c.o");
        assert_eq!(info.objects[0].archive, None);
        assert_eq!(info.objects[0].size, 100);
        assert_eq!(info.objects[0].sections.len(), 7);
        assert_eq!(info.objects[0].sections.get(".bss").unwrap().address, 0);
        assert_eq!(info.objects[0].sections.get(".text").unwrap().size, 22);
        assert_eq!(info.size, 100);
        assert_eq!(info.sections.len(), 7);
        assert_eq!(info.sections[0], 22);
    }

    #[test]
    fn archive() {
        let input = r#"hello.c.o   (ex my libs/libhello.a):
section           size   addr
.text               22      0
.data                0      0
.bss                 0      0
.rodata.str1.1      12      0
.comment            18      0
.note.GNU-stack      0      0
.eh_frame           48      0
Total              100


bye .c.o   (ex my libs/libhello.a):
section           size   addr
.text               12      0
.data                0      0
.bss                 0      0
.rodata.str1.1       5      0
.comment            18      0
.note.GNU-stack      0      0
.eh_frame           48      0
Total               83
"#;
        let info: SizeInfo = input.parse().unwrap();
        assert_eq!(info.objects.len(), 2);
        assert_eq!(info.objects[0].name, "hello.c.o");
        assert_eq!(info.objects[0].archive, Some("my libs/libhello.a".into()));
        assert_eq!(info.objects[0].size, 100);
        assert_eq!(info.objects[1].name, "bye .c.o");
        assert_eq!(info.objects[1].archive, Some("my libs/libhello.a".into()));
        assert_eq!(info.objects[1].size, 83);
        assert_eq!(info.size, 183);
        assert_eq!(info.sections.len(), 7);
        assert_eq!(info.sections[0], 34);
    }
}
