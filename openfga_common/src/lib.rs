use serde::{Deserialize, Serialize};

pub type Span = std::ops::Range<usize>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Access {
    Direct {
        span: Option<Span>,
    },
    Computed {
        object: Identifier,
        relation: Identifier,
        span: Option<Span>,
    },
    SelfComputed {
        relation: Identifier,
        span: Option<Span>,
    },
    Union {
        children: Vec<Access>,
        span: Option<Span>,
    },
    Intersection {
        children: Vec<Access>,
        span: Option<Span>,
    },
    Difference {
        base: Box<Access>,
        subtract: Box<Access>,
        span: Option<Span>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Relation {
    pub name: Identifier,
    pub access: Access,
    pub span: Option<Span>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Type {
    pub name: Identifier,
    pub relations: Vec<Relation>,
    pub span: Option<Span>,
}

impl Type {
    pub fn relation_exists(&self, relation_name: &str) -> bool {
        self.relations.iter().any(|r| r.name.name == relation_name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AuthorizationModel {
    pub types: Vec<Type>,
}

impl AuthorizationModel {
    pub fn type_exists(&self, type_name: &str) -> bool {
        self.types.iter().any(|t| t.name.name == type_name)
    }

    pub fn type_relation_exists(&self, type_name: &str, relation_name: &str) -> bool {
        self.types
            .iter()
            .any(|t| t.name.name == type_name && t.relation_exists(relation_name))
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
                .map(|relation| (relation.name.name, relation.access.into()))
                .collect();
            Type {
                type_name: type_in.name.name,
                relations,
            }
        }
    }

    impl From<Access> for RelationData {
        fn from(access: Access) -> Self {
            match access {
                Access::Direct { span: _ } => RelationData::Direct {
                    this: BTreeMap::new(),
                },
                Access::Union { children, span: _ } => RelationData::Union {
                    union: Usersets {
                        child: children.into_iter().map(|a| a.into()).collect(),
                    },
                },
                Access::Intersection { children, span: _ } => RelationData::Intersection {
                    intersection: Usersets {
                        child: children.into_iter().map(|a| a.into()).collect(),
                    },
                },
                Access::Difference {
                    base,
                    subtract,
                    span: _,
                } => RelationData::Difference {
                    base: Box::new((*base).into()),
                    subtract: Box::new((*subtract).into()),
                },
                Access::SelfComputed { relation, span: _ } => RelationData::ComputedUserset {
                    computed_userset: ObjectRelation {
                        object: "".into(),
                        relation: relation.name,
                    },
                },
                Access::Computed {
                    object,
                    relation,
                    span: _,
                } => RelationData::TupleToUserset {
                    tuple_to_userset: TupleToUserset {
                        tupleset: ObjectRelation {
                            object: "".into(),
                            relation: object.name,
                        },
                        computed_userset: ObjectRelation {
                            object: "".into(),
                            relation: relation.name,
                        },
                    },
                },
            }
        }
    }
}
