use std::collections::HashMap;

use openfga_common::{Access, AuthorizationModel, Relation, Type};

#[derive(Debug)]
pub enum ModelError {
    DuplicateType { name: String },
    DuplicateRelation { name: String, type_name: String },
    UnknownRelation { name: String, type_name: String },
}

fn check_access(
    access: &Access,
    relation: &Relation,
    rtype: &Type,
    model: &AuthorizationModel,
    errors: &mut Vec<ModelError>,
) {
    match access {
        Access::Difference { base, subtract } => {
            check_access(base, relation, rtype, model, errors);
            check_access(subtract, relation, rtype, model, errors);
        }
        Access::Intersection { children } => children
            .iter()
            .for_each(|a| check_access(a, relation, rtype, model, errors)),
        Access::Union { children } => children
            .iter()
            .for_each(|a| check_access(a, relation, rtype, model, errors)),
        Access::SelfComputed {
            relation: relation_name,
        } => {
            if relation_name == &relation.name {
                todo!("Add error for self referencing access rules")
            } else if !rtype.relation_exists(&relation_name) {
                errors.push(ModelError::UnknownRelation {
                    name: relation_name.clone(),
                    type_name: rtype.name.clone(),
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
        if type_map.contains_key(&t.name) {
            errors.push(ModelError::DuplicateType {
                name: t.name.clone(),
            })
        }
        type_map.insert(t.name.clone(), t);

        // Check relations
        let mut relation_map: HashMap<String, &Relation> = HashMap::new();
        t.relations.iter().for_each(|r| {
            // Check for duplicate relation
            if relation_map.contains_key(&r.name) {
                errors.push(ModelError::DuplicateRelation {
                    name: r.name.clone(),
                    type_name: t.name.clone(),
                });
            }
            relation_map.insert(r.name.clone(), r);

            // Check access errors
            check_access(&r.access, r, t, model, &mut errors);
        });
    });

    if errors.len() > 0 {
        return Err(errors);
    }
    Ok(())
}
