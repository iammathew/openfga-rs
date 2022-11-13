use openfga_common::{Access, AuthorizationModel, Identifier, Relation, Type};
use std::{collections::HashMap, ops::Range};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Type with name {} was defined twice", type1.identifier.name)]
    DuplicateTypeName { type1: Type, type2: Type },
    #[error("Relation {} got defined twice on type {}",
    relation1.identifier.name,
    target_type.identifier.name)]
    DuplicateRelationName {
        relation1: Relation,
        relation2: Relation,
        target_type: Type,
    },
    #[error("Relation definition {} on type {} references relation {}, which does not exist",
    relation.identifier.name,
    target_type.identifier.name,
    relation_identifier.name)]
    UnknownRelation {
        relation_identifier: Identifier,
        access: Access,
        relation: Relation,
        target_type: Type,
    },
    #[error("Relation definition {} on type {} references itself",
    relation.identifier.name,
    target_type.identifier.name)]
    SelfReferencingRelation {
        relation_identifier: Identifier,
        access: Access,
        relation: Relation,
        target_type: Type,
    },
}

impl ModelError {
    pub fn get_code(&self) -> u64 {
        match self {
            Self::DuplicateTypeName { type1: _, type2: _ } => 201,
            Self::DuplicateRelationName {
                relation1: _,
                relation2: _,
                target_type: _,
            } => 202,
            Self::UnknownRelation {
                relation_identifier: _,
                access: _,
                relation: _,
                target_type: _,
            } => 203,
            Self::SelfReferencingRelation {
                relation_identifier: _,
                access: _,
                relation: _,
                target_type: _,
            } => 204,
        }
    }

    pub fn get_span(&self) -> Range<usize> {
        match self {
            Self::DuplicateTypeName { type1: _, type2 } => type2.span.clone().unwrap(),
            Self::DuplicateRelationName {
                relation1: _,
                relation2,
                target_type: _,
            } => relation2.span.clone().unwrap(),
            Self::UnknownRelation {
                relation_identifier,
                access: _,
                relation: _,
                target_type: _,
            } => relation_identifier.span.clone().unwrap(),
            Self::SelfReferencingRelation {
                relation_identifier,
                access: _,
                relation: _,
                target_type: _,
            } => relation_identifier.span.clone().unwrap(),
        }
    }
}

fn check_access(
    access: &Access,
    relation: &Relation,
    rtype: &Type,
    model: &AuthorizationModel,
    errors: &mut Vec<ModelError>,
) {
    match access {
        Access::Difference {
            base,
            subtract,
            span: _,
        } => {
            check_access(base, relation, rtype, model, errors);
            check_access(subtract, relation, rtype, model, errors);
        }
        Access::Intersection { children, span: _ } => children
            .iter()
            .for_each(|a| check_access(a, relation, rtype, model, errors)),
        Access::Union { children, span: _ } => children
            .iter()
            .for_each(|a| check_access(a, relation, rtype, model, errors)),
        Access::SelfComputed {
            relation: relation_identifier,
            span: _,
        } => {
            if &relation_identifier.name == &relation.identifier.name {
                errors.push(ModelError::SelfReferencingRelation {
                    relation_identifier: relation_identifier.clone(),
                    access: access.clone(),
                    relation: relation.clone(),
                    target_type: rtype.clone(),
                });
            } else if !rtype.relation_exists(&relation_identifier.name) {
                errors.push(ModelError::UnknownRelation {
                    relation_identifier: relation_identifier.clone(),
                    access: access.clone(),
                    relation: relation.clone(),
                    target_type: rtype.clone(),
                });
            }
        }
        _ => (),
    }
}

pub fn check_model(model: &AuthorizationModel) -> Result<(), Vec<ModelError>> {
    let mut errors: Vec<ModelError> = Vec::new();
    let mut type_map: HashMap<String, &Type> = HashMap::new();
    model.types.iter().for_each(|t| {
        // Check for duplicate type
        if type_map.contains_key(&t.identifier.name) {
            errors.push(ModelError::DuplicateTypeName {
                type1: type_map.get(&t.identifier.name).unwrap().clone().clone(),
                type2: t.clone(),
            })
        }
        type_map.insert(t.identifier.name.clone(), t);

        // Check relations
        let mut relation_map: HashMap<String, &Relation> = HashMap::new();
        t.relations.iter().for_each(|r| {
            // Check for duplicate relation
            if relation_map.contains_key(&r.identifier.name) {
                errors.push(ModelError::DuplicateRelationName {
                    relation1: relation_map
                        .get(&r.identifier.name)
                        .unwrap()
                        .clone()
                        .clone(),
                    relation2: r.clone(),
                    target_type: t.clone(),
                });
            }
            relation_map.insert(r.identifier.name.clone(), r);

            // Check access errors
            check_access(&r.access, r, t, model, &mut errors);
        });
    });

    if errors.len() > 0 {
        return Err(errors);
    }
    Ok(())
}
