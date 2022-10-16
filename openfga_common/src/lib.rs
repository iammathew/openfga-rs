use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Access {
    Direct,
    Computed { object: String, relation: String },
    SelfComputed { relation: String },
    Or(Box<Access>, Box<Access>),
    And(Box<Access>, Box<Access>),
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AuthorizationModel {
    pub types: Vec<Type>,
}

pub mod json {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    use crate::Access;

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct AuthorizationModel {
        pub type_definitions: Vec<Type>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct Type {
        #[serde(rename = "type")]
        pub type_name: String,
        pub relations: HashMap<String, RelationData>,
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
            this: HashMap<String, String>,
        },
        Union {
            union: Usersets,
        },
        Intersection {
            intersection: Usersets,
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
            let relations: HashMap<String, RelationData> = type_in
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
                    this: HashMap::new(),
                },
                Access::Or(access1, access2) => RelationData::Union {
                    union: Usersets {
                        child: vec![(*access1).into(), (*access2).into()],
                    },
                },
                Access::And(access1, access2) => RelationData::Intersection {
                    intersection: Usersets {
                        child: vec![(*access1).into(), (*access2).into()],
                    },
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
