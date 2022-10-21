use std::collections::HashMap;

use openfga_common::{AuthorizationModel, Relation, Type};

#[derive(Debug)]
pub enum ModelError {
    DuplicateType { name: String },
    DuplicateRelation { name: String, type_name: String },
    UnknownRelation { name: String, type_name: String },
}

pub fn check_model(model: &AuthorizationModel) -> Result<(), Vec<ModelError>> {
    let mut errors: Vec<ModelError> = Vec::new();
    let mut type_map: HashMap<String, &Type> = HashMap::new();
    model.types.iter().for_each(|t| {
        if type_map.contains_key(&t.name) {
            errors.push(ModelError::DuplicateType {
                name: t.name.clone(),
            })
        }
        type_map.insert(t.name.clone(), t);
        let mut relation_map: HashMap<String, &Relation> = HashMap::new();
        t.relations.iter().for_each(|r| {
            if relation_map.contains_key(&r.name) {
                errors.push(ModelError::DuplicateRelation {
                    name: r.name.clone(),
                    type_name: t.name.clone(),
                });
            }
            relation_map.insert(r.name.clone(), r);
        });
    });

    if errors.len() > 0 {
        return Err(errors);
    }
    Ok(())
}
