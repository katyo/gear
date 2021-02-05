use crate::{
    Diagnostic, Diagnostics, Error, FixingSuggestion, Location, Result, Severity, TextPoint,
    TextSpan,
};
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, tag, take_while1},
    character::complete::{char, digit1, line_ending, none_of, not_line_ending, space1},
    combinator::{all_consuming, eof, iterator, map, map_res, not, opt, value},
    sequence::{terminated, tuple},
    Err as IErr, IResult,
};
use std::str::FromStr;

impl FromStr for Diagnostics {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        Ok(all_consuming(Self::parse_diagnostics)(input)
            .map_err(|error| match error {
                IErr::Error(error) => error.input,
                IErr::Failure(error) => error.input,
                _ => unreachable!(),
            })
            .map_err(|input| format!("Error while parsing diagnostics: `{}`", input))?
            .1)
    }
}

impl FromStr for Diagnostic {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        Ok(Self::parse_diagnostic(input)
            .map_err(|error| match error {
                IErr::Error(error) => error.input,
                IErr::Failure(error) => error.input,
                _ => unreachable!(),
            })
            .map_err(|input| format!("Error while parsing diagnostic: `{}`", input))?
            .1)
    }
}

impl FromStr for FixingSuggestion {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        Ok(all_consuming(Self::parse_fixit)(input)
            .map_err(|error| match error {
                IErr::Error(error) => error.input,
                IErr::Failure(error) => error.input,
                _ => unreachable!(),
            })
            .map_err(|input| format!("Error while parsing fixing suggestion: `{}`", input))?
            .1)
    }
}

impl FromStr for TextSpan {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        Ok(all_consuming(Self::parse_span)(input)
            .map_err(|error| match error {
                IErr::Error(error) => error.input,
                IErr::Failure(error) => error.input,
                _ => unreachable!(),
            })
            .map_err(|input| format!("Error when parsing text span: `{}`", input))?
            .1)
    }
}

impl FromStr for TextPoint {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        Ok(all_consuming(Self::parse_point)(input)
            .map_err(|error| match error {
                IErr::Error(error) => error.input,
                IErr::Failure(error) => error.input,
                _ => unreachable!(),
            })
            .map_err(|input| format!("Error when parsing text point: `{}`", input))?
            .1)
    }
}

impl Diagnostics {
    fn parse_diagnostics(input: &str) -> IResult<&str, Self> {
        let mut iter = iterator(input, Self::parse_diagnostic_line);
        let diagnostics = iter.filter_map(|v| v).collect();
        let (input, _) = iter.finish()?;
        Ok((input, Self(diagnostics)))
    }

    fn parse_diagnostic_line(input: &str) -> IResult<&str, Option<Diagnostic>> {
        not(eof)(input)?;
        terminated(
            alt((
                map(Diagnostic::parse_diagnostic, Some),
                value(None, not_line_ending),
            )),
            alt((line_ending, eof)),
        )(input)
    }
}

impl Diagnostic {
    fn parse_diagnostic(input: &str) -> IResult<&str, Self> {
        map(
            tuple((
                take_while1(|c| c != ':' && c != '\r' && c != '\n'),
                colon,
                position_number,
                colon,
                position_number,
                colon,
                map(
                    take_while1(|c| c != ':' && c != '\r' && c != '\n'),
                    |severity: &str| Severity::from_str(severity.trim()).unwrap(),
                ),
                colon,
                take_while1(|c| c != '\r' && c != '\n'),
            )),
            |(file, _, line, _, column, _, severity, _, message)| Self {
                severity,
                message: message.trim().into(),
                locations: vec![Location {
                    file: file.into(),
                    point: Some(TextPoint { line, column }),
                    ..Default::default()
                }],
                ..Default::default()
            },
        )(input)
    }
}

impl FixingSuggestion {
    fn parse_fixit(input: &str) -> IResult<&str, Self> {
        map(
            tuple((
                tag("fix-it:"),
                quoted_string,
                tag(":{"),
                TextSpan::parse_span,
                tag("}:"),
                quoted_string,
            )),
            |(_, file, _, span, _, text)| Self { file, span, text },
        )(input)
    }
}

impl TextSpan {
    fn parse_span(input: &str) -> IResult<&str, Self> {
        map(
            tuple((TextPoint::parse_point, char('-'), TextPoint::parse_point)),
            |(start, _, end)| Self { start, end },
        )(input)
    }
}

impl TextPoint {
    fn parse_point(input: &str) -> IResult<&str, Self> {
        map(
            tuple((position_number, char(':'), position_number)),
            |(line, _, column)| Self { line, column },
        )(input)
    }
}

fn colon(input: &str) -> IResult<&str, ()> {
    value((), char(':'))(input)
}

fn quoted_string(input: &str) -> IResult<&str, String> {
    map(
        tuple((
            char('"'),
            escaped_transform(
                none_of(":\"\\ \t\r\n"),
                '\\',
                alt((
                    value(":", char(':')),
                    value("\"", tag("\"")),
                    value("\\", tag("\\")),
                    value(" ", tag(" ")),
                    value("t", tag("\t")),
                    value("r", tag("\r")),
                    value("n", tag("\n")),
                )),
            ),
            char('"'),
        )),
        |(_, data, _)| data,
    )(input)
}

fn position_number(input: &str) -> IResult<&str, u32> {
    map_res(digit1, u32::from_str)(input)
}

fn text_location(input: &str) -> IResult<&str, (Option<u32>, Option<(u32, u32)>)> {
    map(
        tuple((
            opt(tuple((space1, opt(digit1), space1, tag("| ")))),
            take_while1(|c| " ~^".contains(c)),
        )),
        |(_, input): (_, &str)| {
            let point = input.find('^').map(|pos| pos as u32);
            (
                point,
                input.find('~').and_then(|start| {
                    input
                        .rfind('~')
                        .map(|end| (start as u32, end as u32))
                        .map(|(start, end)| {
                            point
                                .map(|point| (start.min(point), end.max(point)))
                                .unwrap_or_else(|| (start, end))
                        })
                }),
            )
        },
    )(input)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fixit_clang() {
        let txt = "fix-it:\"t.cpp\":{7:25-7:29}:\"Gamma\"";
        let fix: FixingSuggestion = txt.parse().unwrap();
        assert_eq!(
            fix,
            FixingSuggestion {
                file: "t.cpp".into(),
                span: TextSpan {
                    start: TextPoint {
                        line: 7,
                        column: 25
                    },
                    end: TextPoint {
                        line: 7,
                        column: 29
                    }
                },
                text: "Gamma".into()
            }
        );
    }

    #[test]
    fn fixit_gcc() {
        let txt = "fix-it:\"test.c\":{45:3-45:21}:\"gtk_widget_show_all\"";
        let fix: FixingSuggestion = txt.parse().unwrap();
        assert_eq!(
            fix,
            FixingSuggestion {
                file: "test.c".into(),
                span: TextSpan {
                    start: TextPoint {
                        line: 45,
                        column: 3
                    },
                    end: TextPoint {
                        line: 45,
                        column: 21
                    }
                },
                text: "gtk_widget_show_all".into()
            }
        );
    }

    #[test]
    fn gcc_le_6() {
        let msg = r#"examples/c/src/main.c:4:20: fatal error: common.h: No such file or directory
 #include "common.h"
                    ^
compilation terminated.

"#;
        let dia: Diagnostic = msg.parse().unwrap();
        assert_eq!(
            dia,
            Diagnostic {
                severity: Severity::Fatal,
                message: "common.h: No such file or directory".into(),
                locations: vec![Location {
                    file: "examples/c/src/main.c".into(),
                    span: None,
                    point: Some(TextPoint {
                        line: 4,
                        column: 20,
                    }),
                    label: None,
                }],
                ..Default::default()
            }
        );
    }

    #[test]
    fn gcc_ge_7() {
        let msg = r#"examples/c/src/main.c:4:10: fatal error: common.h: No such file or directory
 #include "common.h"
          ^~~~~~~~~~
compilation terminated.

"#;
        let dia: Diagnostic = msg.parse().unwrap();
        assert_eq!(
            dia,
            Diagnostic {
                severity: Severity::Fatal,
                message: "common.h: No such file or directory".into(),
                locations: vec![Location {
                    file: "examples/c/src/main.c".into(),
                    span: None,
                    point: Some(TextPoint {
                        line: 4,
                        column: 10,
                    }),
                    label: None,
                }],
                ..Default::default()
            }
        );
    }

    #[test]
    fn clang_ge_5() {
        let msg = r#"examples/c/src/main.c:4:10: fatal error: 'common.h' file not found
#include "common.h"
         ^~~~~~~~~~
1 error generated.

"#;
        let dia: Diagnostic = msg.parse().unwrap();
        assert_eq!(
            dia,
            Diagnostic {
                severity: Severity::Fatal,
                message: "'common.h' file not found".into(),
                locations: vec![Location {
                    file: "examples/c/src/main.c".into(),
                    span: None,
                    point: Some(TextPoint {
                        line: 4,
                        column: 10,
                    }),
                    label: None,
                }],
                ..Default::default()
            }
        );
    }

    #[test]
    fn diag_single() {
        let msg = r#"examples/c/src/main.c:4:10: fatal error: 'common.h' file not found
#include "common.h"
         ^~~~~~~~~~
examples/c/src/main.c:5:10: fatal error: 'bye.h' file not found
#include "bye.h"
         ^~~~~~~

examples/c/src/main.c:6:10: fatal error: 'hello.h' file not found
#include "hello.h"
         ^~~~~~~~~

3 errors generated.

"#;
        let dia: Diagnostics = msg.parse().unwrap();
        assert_eq!(
            dia,
            Diagnostics(vec![
                Diagnostic {
                    severity: Severity::Fatal,
                    message: "'common.h' file not found".into(),
                    locations: vec![Location {
                        file: "examples/c/src/main.c".into(),
                        span: None,
                        point: Some(TextPoint {
                            line: 4,
                            column: 10,
                        }),
                        label: None,
                    }],
                    ..Default::default()
                },
                Diagnostic {
                    severity: Severity::Fatal,
                    message: "'bye.h' file not found".into(),
                    locations: vec![Location {
                        file: "examples/c/src/main.c".into(),
                        span: None,
                        point: Some(TextPoint {
                            line: 5,
                            column: 10,
                        }),
                        label: None,
                    }],
                    ..Default::default()
                },
                Diagnostic {
                    severity: Severity::Fatal,
                    message: "'hello.h' file not found".into(),
                    locations: vec![Location {
                        file: "examples/c/src/main.c".into(),
                        span: None,
                        point: Some(TextPoint {
                            line: 6,
                            column: 10,
                        }),
                        label: None,
                    }],
                    ..Default::default()
                },
            ]),
        );
    }

    #[test]
    fn diag_multiple() {
        let msg = r#"examples/c/src/main.c:4:10: fatal error: 'common.h' file not found
#include "common.h"
         ^~~~~~~~~~
1 error generated.

"#;
        let dia: Diagnostics = msg.parse().unwrap();
        assert_eq!(
            dia,
            Diagnostics(vec![Diagnostic {
                severity: Severity::Fatal,
                message: "'common.h' file not found".into(),
                locations: vec![Location {
                    file: "examples/c/src/main.c".into(),
                    span: None,
                    point: Some(TextPoint {
                        line: 4,
                        column: 10,
                    }),
                    label: None,
                }],
                ..Default::default()
            },]),
        );
    }

    #[test]
    fn caret_only() {
        let msg = "     ^";
        assert_eq!(text_location(msg).unwrap(), ("", (Some(5), None)));
    }

    #[test]
    fn span_only() {
        let msg = "   ~~~~~";
        assert_eq!(text_location(msg).unwrap(), ("", (None, Some((3, 7)))));
    }

    #[test]
    fn separated_span() {
        let msg = "   ~~  ~~~";
        assert_eq!(text_location(msg).unwrap(), ("", (None, Some((3, 9)))));
    }

    #[test]
    fn span_with_caret_at_end() {
        let msg = "    ~~~^";
        assert_eq!(text_location(msg).unwrap(), ("", (Some(7), Some((4, 7)))));
    }

    #[test]
    fn span_with_caret_at_start() {
        let msg = "    ^~~~~";
        assert_eq!(text_location(msg).unwrap(), ("", (Some(4), Some((4, 8)))));
    }

    #[test]
    fn span_with_caret_into() {
        let msg = "    ~~^~~~";
        assert_eq!(text_location(msg).unwrap(), ("", (Some(6), Some((4, 9)))));
    }

    #[test]
    fn separated_span_with_caret_into() {
        let msg = "    ~~ ^ ~~~";
        assert_eq!(text_location(msg).unwrap(), ("", (Some(7), Some((4, 11)))));
    }
}
