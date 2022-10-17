use std::fmt;

use chumsky::{prelude::*, stream::Stream};
use openfga_common::{Access, Relation, Type};

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
            Token::Identifier(s) => write!(f, "{}", s),
        }
    }
}

pub fn lexer() -> impl Parser<char, Vec<Spanned<Token>>, Error = Simple<char>> {
    let token = text::ident()
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
            _ => Token::Identifier(ident),
        })
        .labelled("token");

    let comment = just("//")
        .then(take_until(just('\n')))
        .padded()
        .labelled("comment");

    token
        .map_with_span(|tok, span| (tok, span))
        .padded_by(comment.repeated())
        .padded()
        .repeated()
        .then_ignore(end())
}

pub fn better_parser() -> impl Parser<Token, Vec<Type>, Error = Simple<Token>> + Clone {
    let ident = select! { Token::Identifier(ident) => ident.clone() }.labelled("identifier");
    let direct_access = just(Token::SelfRef)
        .map(|_| Access::Direct)
        .labelled("direct access");

    let computed_self_access = ident
        .map(|relation| Access::Computed {
            object: String::from(""),
            relation,
        })
        .labelled("computed self access");

    let computed_relation_access = ident
        .then_ignore(just(Token::From))
        .then(ident)
        .map(|(relation, object)| Access::Computed { object, relation })
        .labelled("computed relation access");

    let simple_access = choice((
        direct_access,
        computed_relation_access,
        computed_self_access,
    ))
    .labelled("access");

    let and_access = simple_access
        .separated_by(just(Token::And))
        .map(|accesses| {
            accesses
                .into_iter()
                .reduce(|prev, current| Access::And(Box::new(current), Box::new(prev)))
                .unwrap()
        })
        .labelled("and");

    let or_access = and_access
        .separated_by(just(Token::Or))
        .map(|accesses| {
            accesses
                .into_iter()
                .reduce(|prev, current| Access::Or(Box::new(current), Box::new(prev)))
                .unwrap()
        })
        .labelled("or");

    let relation = just(Token::Define)
        .ignore_then(ident)
        .then_ignore(just(Token::As))
        .then(or_access)
        .map(|(name, access)| Relation {
            name: name,
            access: access,
        })
        .labelled("relation");

    let relations = just(Token::Relations)
        .ignore_then(relation.repeated())
        .labelled("relations");

    let typep = just(Token::Type)
        .ignore_then(ident)
        .then(relations)
        .map(|(ident, relations)| Type {
            name: ident,
            relations,
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
    use crate::parse_model;
    use openfga_common::{Access, Relation, Type};

    #[test]
    fn parses_type() {
        let src = concat!(
            "type test\n",
            "    relations\n",
            "       define test as self\n"
        );
        let out = parse_model(src.trim()).unwrap();
        assert_eq!(
            out,
            vec![Type {
                name: String::from("test"),
                relations: vec![Relation {
                    name: String::from("test"),
                    access: Access::Direct
                }]
            }]
        );
    }
}
