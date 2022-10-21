use ariadne::{sources, Color, Fmt, Label, Report, ReportKind};
use chumsky::{prelude::*, stream::Stream};
use openfga_checker::check_model;
use openfga_common::json::AuthorizationModel as JsonAuthModel;
use openfga_common::AuthorizationModel;
use openfga_dsl_parser::{better_parser, lexer};
use std::{env, fs, path::Path};

fn main() {
    let path_string: String = env::args().nth(1).expect("Expected file argument");
    let path = Path::new(&path_string);
    let src = fs::read_to_string(&path).expect("Failed to read file");

    let (tokens, _errors) = lexer().parse_recovery_verbose(src.trim());
    let len = src.chars().count();
    let (types, errs) = better_parser()
        .parse_recovery_verbose(Stream::from_iter(len..len + 1, tokens.unwrap().into_iter()));
    match types {
        Some(types) => {
            let model = AuthorizationModel { types };
            let res = check_model(&model);
            match res {
                Ok(()) => {
                    let json_model: JsonAuthModel = model.into();
                    let json = serde_json::to_string_pretty(&json_model);
                    match json {
                        Ok(string) => println!("{}", string),
                        Err(err) => println!("{}", err),
                    }
                }
                Err(errors) => {
                    println!("{:?}", errors);
                }
            }
        }
        None => {
            errs.into_iter().for_each(|e| {
                let msg = if let chumsky::error::SimpleReason::Custom(msg) = e.reason() {
                    msg.clone()
                } else {
                    format!(
                        "{}{}, expected {}",
                        if e.found().is_some() {
                            "Unexpected token"
                        } else {
                            "Unexpected end of input"
                        },
                        if let Some(label) = e.label() {
                            format!(" while parsing {}", label)
                        } else {
                            String::new()
                        },
                        if e.expected().len() == 0 {
                            "something else".to_string()
                        } else {
                            e.expected()
                                .map(|expected| match expected {
                                    Some(expected) => expected.to_string(),
                                    None => "end of input".to_string(),
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        },
                    )
                };

                let report = Report::build(ReportKind::Error, path_string.clone(), e.span().start)
                    .with_code(3)
                    .with_message(msg)
                    .with_label(
                        Label::new((path_string.clone(), e.span()))
                            .with_message(match e.reason() {
                                chumsky::error::SimpleReason::Custom(msg) => msg.clone(),
                                _ => format!(
                                    "Unexpected {}",
                                    e.found()
                                        .map(|c| format!("token {}", c.fg(Color::Red)))
                                        .unwrap_or_else(|| "end of input".to_string())
                                ),
                            })
                            .with_color(Color::Red),
                    );

                let report = match e.reason() {
                    chumsky::error::SimpleReason::Unclosed { span, delimiter } => report
                        .with_label(
                            Label::new((path_string.clone(), span.clone()))
                                .with_message(format!(
                                    "Unclosed delimiter {}",
                                    delimiter.fg(Color::Yellow)
                                ))
                                .with_color(Color::Yellow),
                        ),
                    chumsky::error::SimpleReason::Unexpected => report,
                    chumsky::error::SimpleReason::Custom(_) => report,
                };

                report
                    .finish()
                    .print(sources(vec![(path_string.clone(), src.clone())]))
                    .unwrap();
            });
        }
    }
}
