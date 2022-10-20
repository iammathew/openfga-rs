use std::collections::HashMap;

use openfga_common::{AuthorizationModel, Type};

pub enum ModelError {
    DuplicateType { name: String },
    DuplicateRelation { name: String, typeName: String },
    UnknownRelation { name: String, typeName: String },
}

pub fn check_model(model: &AuthorizationModel) -> Result<(), Vec<ModelError>> {
    let mut errors: Vec<ModelError> = Vec::new();
    let map: HashMap<String, &Type> = HashMap::new();
    model.types.iter().for_each(|t| {
        if map.contains_key(&t.name) {
            errors.push(ModelError::DuplicateType {
                name: t.name.clone(),
            })
        }
    });

    if errors.len() == 0 {
        return Err(errors);
    }
    Ok(())
}
