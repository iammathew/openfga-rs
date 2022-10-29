use std::{fmt, ops::Range};

use chumsky::{prelude::*, stream::Stream};
use openfga_common::{Access, Identifier, Relation, Type};

pub type Span = std::ops::Range<usize>;
pub type Spanned<T> = (T, Span);

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Token {
    Type,
    Relations,
    SelfRef,
    Define,
    And,
    Or,
    From,
    As,
    But,
    Not,
    OpenParenthesis,
    CloseParenthesis,
    Identifier(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Type => write!(f, "type"),
            Token::Relations => write!(f, "relations"),
            Token::SelfRef => write!(f, "self"),
            Token::Define => write!(f, "define"),
            Token::And => write!(f, "and"),
            Token::Or => write!(f, "or"),
            Token::From => write!(f, "from"),
            Token::As => write!(f, "as"),
            Token::But => write!(f, "but"),
            Token::Not => write!(f, "not"),
            Token::OpenParenthesis => write!(f, "("),
            Token::CloseParenthesis => write!(f, ")"),
            Token::Identifier(s) => write!(f, "{}", s),
        }
    }
}

pub fn lexer() -> impl Parser<char, Vec<Spanned<Token>>, Error = Simple<char>> {
    let ctrl = one_of("()").map(|c| match c {
        '(' => Token::OpenParenthesis,
        ')' => Token::CloseParenthesis,
        _ => panic!("IMPOSSIBLE!"),
    });

    let ident = text::ident()
        .map(|ident: String| match ident.as_str() {
            "type" => Token::Type,
            "relations" => Token::Relations,
            "self" => Token::SelfRef,
            "define" => Token::Define,
            "and" => Token::And,
            "or" => Token::Or,
            "from" => Token::From,
            "as" => Token::As,
            "but" => Token::But,
            "not" => Token::Not,
            "(" => Token::OpenParenthesis,
            ")" => Token::CloseParenthesis,
            _ => Token::Identifier(ident),
        })
        .labelled("token");

    let comment = just("//")
        .then(take_until(just('\n')))
        .padded()
        .labelled("comment");

    let token = ctrl.or(ident);

    token
        .map_with_span(|tok, span| (tok, span))
        .padded_by(comment.repeated())
        .padded()
        .repeated()
        .then_ignore(end())
}

pub fn better_parser() -> impl Parser<Token, Vec<Type>, Error = Simple<Token>> + Clone {
    let ident = select! { Token::Identifier(ident) => ident.clone() }
        .map_with_span(|name, span| Identifier {
            name,
            span: Some(span),
        })
        .labelled("identifier");

    let access = recursive(|access| {
        let parenthesis_access =
            access.delimited_by(just(Token::OpenParenthesis), just(Token::CloseParenthesis));

        let direct_access = just(Token::SelfRef)
            .map_with_span(|_, span| Access::Direct { span: Some(span) })
            .labelled("direct access");

        let computed_self_access = ident
            .map_with_span(|relation, span| Access::SelfComputed {
                relation,
                span: Some(span),
            })
            .labelled("computed self access");

        let computed_relation_access = ident
            .then_ignore(just(Token::From))
            .then(ident)
            .map_with_span(|(relation, object), span| Access::Computed {
                object,
                relation,
                span: Some(span),
            })
            .labelled("computed relation access");

        let simple_access = choice((
            direct_access,
            parenthesis_access,
            computed_relation_access,
            computed_self_access,
        ))
        .labelled("simple access");

        let difference_access = simple_access
            .separated_by(just(Token::But).then(just(Token::Not)))
            .at_least(1)
            .at_most(2)
            .map_with_span(|accesses: Vec<Access>, span: Range<usize>| {
                accesses
                    .into_iter()
                    .reduce(|prev, current| Access::Difference {
                        base: Box::new(prev),
                        subtract: Box::new(current),
                        span: Some(span.clone()),
                    })
                    .unwrap()
            })
            .labelled("but not");

        let and_access = difference_access
            .separated_by(just(Token::And))
            .at_least(1)
            .map_with_span(|mut accesses, span| {
                if accesses.len() == 1 {
                    return accesses.pop().unwrap();
                }
                Access::Intersection {
                    children: accesses,
                    span: Some(span),
                }
            })
            .labelled("and");

        let or_access = and_access
            .separated_by(just(Token::Or))
            .at_least(1)
            .map_with_span(|mut accesses, span| {
                if accesses.len() == 1 {
                    return accesses.pop().unwrap();
                }
                Access::Union {
                    children: accesses,
                    span: Some(span),
                }
            })
            .labelled("or");

        return or_access;
    });

    let relation = just(Token::Define)
        .ignore_then(ident)
        .then_ignore(just(Token::As))
        .then(access)
        .map_with_span(|(name, access), span| Relation {
            name: name,
            access: access,
            span: Some(span),
        })
        .labelled("relation");

    let relations = just(Token::Relations)
        .ignore_then(relation.repeated())
        .labelled("relations");

    let typep = just(Token::Type)
        .ignore_then(ident)
        .then(relations)
        .map_with_span(|(ident, relations), span| Type {
            name: ident,
            relations,
            span: Some(span),
        })
        .labelled("type");

    typep.repeated().then_ignore(end())
}

#[derive(Debug)]
pub enum ParseErrors {
    Lexer(Vec<Simple<char>>),
    Parser(Vec<Simple<Token>>),
}

pub fn parse_model(src: &str) -> Result<Vec<Type>, ParseErrors> {
    let (tokens, errors) = lexer().parse_recovery_verbose(src.trim());
    if tokens.is_none() {
        return Err(ParseErrors::Lexer(errors));
    }
    let len = src.chars().count();
    let (types, errs) = better_parser()
        .parse_recovery_verbose(Stream::from_iter(len..len + 1, tokens.unwrap().into_iter()));
    if types.is_none() {
        return Err(ParseErrors::Parser(errs));
    }
    Ok(types.unwrap())
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn parses_type() {
    //     let src = concat!(
    //         "type test\n",
    //         "    relations\n",
    //         "       define test as self\n"
    //     );
    //     let out = parse_model(src.trim()).unwrap();
    //     assert_eq!(
    //         out,
    //         vec![Type {
    //             name: String::from("test"),
    //             relations: vec![Relation {
    //                 name: String::from("test"),
    //                 access: Access::Direct
    //             }]
    //         }]
    //     );
    // }
}
