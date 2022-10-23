use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Access {
    Direct,
    Computed {
        object: String,
        relation: String,
    },
    SelfComputed {
        relation: String,
    },
    Union {
        children: Vec<Access>,
    },
    Intersection {
        children: Vec<Access>,
    },
    Difference {
        base: Box<Access>,
        subtract: Box<Access>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Relation {
    pub name: String,
    pub access: Access,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Type {
    pub name: String,
    pub relations: Vec<Relation>,
}

impl Type {
    pub fn relation_exists(&self, relation_name: &str) -> bool {
        self.relations.iter().any(|r| r.name == relation_name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AuthorizationModel {
    pub types: Vec<Type>,
}

impl AuthorizationModel {
    pub fn type_exists(&self, type_name: &str) -> bool {
        self.types.iter().any(|t| t.name == type_name)
    }

    pub fn type_relation_exists(&self, type_name: &str, relation_name: &str) -> bool {
        self.types
            .iter()
            .any(|t| t.name == type_name && t.relation_exists(relation_name))
    }
}

pub mod json {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    use crate::Access;

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct AuthorizationModel {
        pub type_definitions: Vec<Type>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct Type {
        #[serde(rename = "type")]
        pub type_name: String,
        pub relations: BTreeMap<String, RelationData>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct Usersets {
        pub child: Vec<RelationData>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct ObjectRelation {
        pub object: String,
        pub relation: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct TupleToUserset {
        pub tupleset: ObjectRelation,
        #[serde(rename = "computedUserset")]
        pub computed_userset: ObjectRelation,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    #[serde(untagged)]
    pub enum RelationData {
        Direct {
            this: BTreeMap<String, String>,
        },
        Union {
            union: Usersets,
        },
        Intersection {
            intersection: Usersets,
        },
        Difference {
            base: Box<RelationData>,
            subtract: Box<RelationData>,
        },
        TupleToUserset {
            #[serde(rename = "tupleToUserset")]
            tuple_to_userset: TupleToUserset,
        },
        ComputedUserset {
            #[serde(rename = "computedUserset")]
            computed_userset: ObjectRelation,
        },
    }

    impl From<super::AuthorizationModel> for AuthorizationModel {
        fn from(model: super::AuthorizationModel) -> Self {
            AuthorizationModel {
                type_definitions: model.types.into_iter().map(|t| t.into()).collect(),
            }
        }
    }

    impl From<super::Type> for Type {
        fn from(type_in: super::Type) -> Self {
            let relations: BTreeMap<String, RelationData> = type_in
                .relations
                .into_iter()
                .map(|relation| (relation.name, relation.access.into()))
                .collect();
            Type {
                type_name: type_in.name,
                relations,
            }
        }
    }

    impl From<Access> for RelationData {
        fn from(access: Access) -> Self {
            match access {
                Access::Direct => RelationData::Direct {
                    this: BTreeMap::new(),
                },
                Access::Union { children } => RelationData::Union {
                    union: Usersets {
                        child: children.into_iter().map(|a| a.into()).collect(),
                    },
                },
                Access::Intersection { children } => RelationData::Intersection {
                    intersection: Usersets {
                        child: children.into_iter().map(|a| a.into()).collect(),
                    },
                },
                Access::Difference { base, subtract } => RelationData::Difference {
                    base: Box::new((*base).into()),
                    subtract: Box::new((*subtract).into()),
                },
                Access::SelfComputed { relation } => RelationData::ComputedUserset {
                    computed_userset: ObjectRelation {
                        object: "".into(),
                        relation,
                    },
                },
                Access::Computed { object, relation } => RelationData::TupleToUserset {
                    tuple_to_userset: TupleToUserset {
                        tupleset: ObjectRelation {
                            object: "".into(),
                            relation: object,
                        },
                        computed_userset: ObjectRelation {
                            object: "".into(),
                            relation: relation,
                        },
                    },
                },
            }
        }
    }
}
