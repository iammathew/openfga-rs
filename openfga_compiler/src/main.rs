use ariadne::{sources, Color, Fmt, Label, Report, ReportKind};
use chumsky::{prelude::*, stream::Stream};
use clap::Parser as CliParser;
use openfga_checker::{check_model, ModelError};
use openfga_common::json::AuthorizationModel as JsonAuthModel;
use openfga_common::AuthorizationModel;
use openfga_dsl_parser::{better_parser, lexer, Token};
use std::{fs, path::PathBuf};

#[derive(CliParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// input path of dsl model file
    input_file: PathBuf,

    /// output path of compiled model
    #[arg(short, long)]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();
    let src = fs::read_to_string(&args.input_file).expect("Failed to read file");
    let path_string = args.input_file.into_os_string().into_string().unwrap();

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
                        Ok(string) => fs::write(&args.output, string).expect("Write failed!"),
                        Err(err) => println!("Error serializing model to json: {}", err),
                    }
                }
                Err(errors) => {
                    errors.into_iter().for_each(|e| {
                        let report = build_model_error_report(e, &path_string);

                        report
                            .finish()
                            .print(sources(vec![(path_string.clone(), src.clone())]))
                            .unwrap();
                    });
                }
            }
        }
        None => {
            errs.into_iter().for_each(|e| {
                let report = build_simple_token_report(e, &path_string);

                report
                    .finish()
                    .print(sources(vec![(path_string.clone(), src.clone())]))
                    .unwrap();
            });
        }
    }
}

fn build_model_error_report(
    e: ModelError,
    path_string: &String,
) -> ariadne::ReportBuilder<(String, std::ops::Range<usize>)> {
    match e {
        ModelError::DuplicateTypeName { type1, type2 } => Report::build(
            ReportKind::Error,
            path_string.clone(),
            type1.span.clone().unwrap().start,
        )
        .with_code(201)
        .with_message(format!(
            "Type with name {} got defined twice",
            type1.identifier.name.clone().fg(Color::Blue)
        ))
        .with_label(
            Label::new((path_string.clone(), type1.span.unwrap().clone()))
                .with_message(format!(
                    "Type with name {} first was defined here",
                    type1.identifier.name.clone().fg(Color::Blue)
                ))
                .with_color(Color::Red),
        )
        .with_label(
            Label::new((path_string.clone(), type2.span.unwrap().clone()))
                .with_message(format!("and later here again!"))
                .with_color(Color::Red),
        )
        .with_note("The names of type must be unique inside the model"),

        ModelError::DuplicateRelationName {
            relation1,
            relation2,
            target_type,
        } => Report::build(
            ReportKind::Error,
            path_string.clone(),
            target_type.span.clone().unwrap().start,
        )
        .with_code(202)
        .with_message(format!(
            "Relation {} got defined twice on type {}",
            relation1.identifier.name.clone().fg(Color::Blue),
            target_type.identifier.name.clone().fg(Color::Blue)
        ))
        .with_label(
            Label::new((path_string.clone(), relation1.span.unwrap().clone()))
                .with_message(format!(
                    "Relation {} first was defined here",
                    relation1.identifier.name.clone().fg(Color::Blue)
                ))
                .with_color(Color::Red),
        )
        .with_label(
            Label::new((path_string.clone(), relation2.span.unwrap().clone()))
                .with_message(format!("and again here"))
                .with_color(Color::Red),
        )
        .with_label(
            Label::new((path_string.clone(), target_type.span.unwrap().clone()))
                .with_message(format!(
                    "on type {}",
                    target_type.identifier.name.clone().fg(Color::Blue)
                ))
                .with_color(Color::Blue),
        )
        .with_note("The names of relations must be unique inside a single type"),

        ModelError::UnknownRelation {
            relation_identifier: relation_name,
            access: _,
            relation,
            target_type,
        } => Report::build(
            ReportKind::Error,
            path_string.clone(),
            target_type.span.clone().unwrap().start,
        )
        .with_code(203)
        .with_message(format!(
            "Relation definition {} on type {} references relation {}, which does not exist",
            relation.identifier.name.clone().fg(Color::Green),
            target_type.identifier.name.clone().fg(Color::Blue),
            relation_name.name.clone().fg(Color::Green),
        ))
        .with_label(
            Label::new((path_string.clone(), relation_name.span.unwrap().clone()))
                .with_message(format!(
                    "Relation {} mentioned here does not exist on type {}",
                    relation_name.name.clone().fg(Color::Green),
                    target_type.identifier.name.clone().fg(Color::Blue)
                ))
                .with_color(Color::Red),
        )
        .with_note("Relations actually do need to exist"),

        ModelError::SelfReferencingRelation {
            relation_identifier: relation_name,
            access: _,
            relation,
            target_type,
        } => Report::build(
            ReportKind::Error,
            path_string.clone(),
            target_type.span.clone().unwrap().start,
        )
        .with_code(204)
        .with_message(format!(
            "Relation definition {} on type {} references itself",
            relation.identifier.name.clone().fg(Color::Green),
            target_type.identifier.name.clone().fg(Color::Blue),
        ))
        .with_label(
            Label::new((path_string.clone(), relation_name.span.unwrap().clone()))
                .with_message(format!(
                    "Relation {} mentions itself here",
                    relation_name.name.clone().fg(Color::Green),
                ))
                .with_color(Color::Red),
        )
        .with_note("Relations can't reference themselves"),

        _ => Report::build(ReportKind::Error, path_string.clone(), 0),
    }
}

fn build_simple_token_report(
    e: Simple<Token>,
    path_string: &String,
) -> ariadne::ReportBuilder<(String, std::ops::Range<usize>)> {
    let msg = get_simple_token_error_message(&e);
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
        chumsky::error::SimpleReason::Unclosed { span, delimiter } => report.with_label(
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
}

fn get_simple_token_error_message(e: &Simple<Token>) -> String {
    let msg = if let chumsky::error::SimpleReason::Custom(msg) = e.reason() {
        msg.clone()
    } else {
        format!(
            "{}{}, expected instead {}",
            match e.found() {
                Some(f) => format!("Found unexpected token {}", f.fg(Color::Blue).to_string()),
                None => format!(
                    "Found unexpected {}",
                    String::from("end of input").fg(Color::Blue).to_string()
                ),
            },
            if let Some(label) = e.label() {
                format!(" while parsing {}", label.fg(Color::Green).to_string())
            } else {
                String::new()
            },
            if e.expected().len() == 0 {
                "something else".to_string()
            } else {
                e.expected()
                    .map(|expected| match expected {
                        Some(expected) => expected.fg(Color::Blue).to_string(),
                        None => "end of input".fg(Color::Blue).to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        )
    };
    return msg;
}
