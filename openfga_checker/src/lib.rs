use std::collections::HashMap;

use openfga_common::{Access, AuthorizationModel, Identifier, Relation, Type};

#[derive(Debug)]
pub enum ModelError {
    DuplicateTypeName {
        type1: Type,
        type2: Type,
    },
    DuplicateRelationName {
        relation1: Relation,
        relation2: Relation,
        target_type: Type,
    },
    UnknownRelation {
        relation_identifier: Identifier,
        access: Access,
        relation: Relation,
        target_type: Type,
    },
    SelfReferencingRelation {
        relation_identifier: Identifier,
        access: Access,
        relation: Relation,
        target_type: Type,
    },
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
