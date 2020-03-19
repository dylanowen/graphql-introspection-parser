use graphql_parser::schema::{
    Definition, Document, EnumType, EnumValue, Field, InputObjectType, InputValue, InterfaceType,
    NamedType, ObjectType, ScalarType, SchemaDefinition, Type, TypeDefinition, UnionType,
};
use graphql_parser::Pos;
use serde::de::{self, Deserializer, IgnoredAny, MapAccess, Unexpected, Visitor};
use serde::Deserialize;
use serde_json as json;
use std::fmt;

const QUERY_TYPE_ALIAS: &str = "queryType";
const MUTATION_TYPE_ALIAS: &str = "mutationType";
const SUBSCRIPTION_TYPE_ALIAS: &str = "subscriptionType";
const TYPES_ALIAS: &str = "types";
const DIRECTIVES_ALIAS: &str = "directives";

const KIND_ALIAS: &str = "kind";
const NAME_ALIAS: &str = "name";
const DESCRIPTION_ALIAS: &str = "description";
const FIELDS_ALIAS: &str = "fields";
const INPUT_FIELDS_ALIAS: &str = "inputFields";
const INTERFACES_ALIAS: &str = "interfaces";
const ENUM_VALUES_ALIAS: &str = "enumValues";
const POSSIBLE_TYPES_ALIAS: &str = "possibleTypes";
const ARGS_ALIAS: &str = "args";
const TYPE_ALIAS: &str = "type";
const DEFAULT_VALUE_ALIAS: &str = "defaultValue";
const OF_TYPE_ALIAS: &str = "ofType";
const IS_DEPRECATED_ALIAS: &str = "isDeprecated";
const DEPRECATION_REASON_ALIAS: &str = "deprecationReason";

pub fn parse(raw_introspection: &str) -> serde_json::Result<Document> {
    serde_json::from_str::<ResponseContainer>(raw_introspection).map(|c| c.data.schema)
}

#[derive(Deserialize)]
struct ResponseContainer {
    data: SchemaContainer,
}

#[derive(Deserialize)]
struct SchemaContainer {
    #[serde(
        rename(deserialize = "__schema"),
        deserialize_with = "deserialize_document"
    )]
    schema: Document,
}

fn deserialize_document<'de, D>(deserializer: D) -> Result<Document, D::Error>
where
    D: Deserializer<'de>,
{
    struct DocumentVisitor;

    fn deserialize_root_type<'de, M>(
        previous_result: &Option<NamedType>,
        alias: &'static str,
        access: &mut M,
    ) -> Result<Option<String>, M::Error>
    where
        M: MapAccess<'de>,
    {
        if previous_result.is_none() {
            access
                .next_value::<json::Value>()
                .and_then(|value| match value {
                    json::Value::Null => Ok(None),
                    json::Value::Object(map) => map
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| Some(s.to_string()))
                        .ok_or_else(|| de::Error::missing_field("name")),
                    _ => Err(de::Error::invalid_type(
                        Unexpected::Other(&format!("{}", value)),
                        &"object type",
                    )),
                })
        } else {
            Err(de::Error::duplicate_field(alias))
        }
    }

    impl<'de> Visitor<'de> for DocumentVisitor {
        type Value = Document;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("A Document object")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut query_type = None;
            let mut mutation_type = None;
            let mut subscription_type = None;
            let mut types: Vec<Definition> = vec![];

            while let Some(key) = access.next_key()? {
                match key {
                    QUERY_TYPE_ALIAS => {
                        query_type =
                            deserialize_root_type(&query_type, QUERY_TYPE_ALIAS, &mut access)?;
                    }
                    MUTATION_TYPE_ALIAS => {
                        mutation_type = deserialize_root_type(
                            &mutation_type,
                            MUTATION_TYPE_ALIAS,
                            &mut access,
                        )?;
                    }
                    SUBSCRIPTION_TYPE_ALIAS => {
                        subscription_type = deserialize_root_type(
                            &subscription_type,
                            SUBSCRIPTION_TYPE_ALIAS,
                            &mut access,
                        )?;
                    }
                    DIRECTIVES_ALIAS => {
                        access.next_value::<IgnoredAny>()?;
                    }
                    TYPES_ALIAS => {
                        types = access
                            .next_value::<Vec<DeserializeWith<TypeDefinition>>>()?
                            .into_iter()
                            .map(|v| Definition::TypeDefinition(v.value))
                            .collect();
                    }
                    _ => handle_unexpected_key(key, &mut access)?,
                }
            }

            let schema_definition = Definition::SchemaDefinition(SchemaDefinition {
                position: Pos::default(),
                directives: vec![],
                query: query_type,
                mutation: mutation_type,
                subscription: subscription_type,
            });

            // build up our final definitions vec
            let mut definitions = types;
            definitions.push(schema_definition);

            Ok(Document { definitions })
        }
    }

    deserializer.deserialize_map(DocumentVisitor)
}

impl<'de> Deserialize<'de> for DeserializeWith<TypeDefinition> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_type_definition(deserializer).map(|value| DeserializeWith { value })
    }
}

fn deserialize_type_definition<'de, D>(deserializer: D) -> Result<TypeDefinition, D::Error>
where
    D: Deserializer<'de>,
{
    struct TypeDefinitionVisitor;

    impl<'de> Visitor<'de> for TypeDefinitionVisitor {
        type Value = TypeDefinition;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("A TypeDefinition object")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut kind: Option<TypeKind> = None;
            let mut maybe_name: Option<String> = None;
            let mut description: Option<String> = None;
            let mut fields: Option<Vec<Field>> = None;
            let mut input_fields: Option<Vec<InputValue>> = None;
            let mut interfaces: Option<Vec<NamedType>> = None;
            let mut enum_values: Option<Vec<EnumValue>> = None;
            let mut possible_types: Option<Vec<NamedType>> = None;

            while let Some(key) = access.next_key()? {
                match key {
                    KIND_ALIAS => {
                        kind = Some(access.next_value()?);
                    }
                    NAME_ALIAS => {
                        maybe_name = Some(access.next_value()?);
                    }
                    DESCRIPTION_ALIAS => {
                        description = access.next_value()?;
                    }
                    FIELDS_ALIAS => {
                        fields = DeserializeWith::deserialize_array(&mut access)?;
                    }
                    INPUT_FIELDS_ALIAS => {
                        input_fields = DeserializeWith::deserialize_array(&mut access)?;
                    }
                    INTERFACES_ALIAS => {
                        interfaces = DeserializeWith::deserialize_array(&mut access)?;
                    }
                    ENUM_VALUES_ALIAS => {
                        enum_values = DeserializeWith::deserialize_array(&mut access)?;
                    }
                    POSSIBLE_TYPES_ALIAS => {
                        possible_types = DeserializeWith::deserialize_array(&mut access)?;
                    }
                    _ => handle_unexpected_key(key, &mut access)?,
                }
            }

            // all of our types need a name
            let name = require_field(NAME_ALIAS, maybe_name)?;

            let result = match require_field(KIND_ALIAS, kind)? {
                TypeKind::Scalar => {
                    require_field_empty(FIELDS_ALIAS, fields)?;
                    require_field_empty(INPUT_FIELDS_ALIAS, input_fields)?;
                    require_field_empty(INTERFACES_ALIAS, interfaces)?;
                    require_field_empty(POSSIBLE_TYPES_ALIAS, possible_types)?;

                    TypeDefinition::Scalar(ScalarType {
                        position: Pos::default(),
                        description,
                        name,
                        directives: vec![],
                    })
                }
                TypeKind::Object => {
                    require_field_empty(INPUT_FIELDS_ALIAS, input_fields)?;
                    require_field_empty(POSSIBLE_TYPES_ALIAS, possible_types)?;

                    TypeDefinition::Object(ObjectType {
                        position: Pos::default(),
                        description,
                        name,
                        implements_interfaces: interfaces.unwrap_or_else(|| vec![]),
                        directives: vec![],
                        fields: fields.unwrap_or_else(|| vec![]),
                    })
                }
                TypeKind::Interface => {
                    require_field_empty(INPUT_FIELDS_ALIAS, input_fields)?;
                    require_field_empty(INTERFACES_ALIAS, interfaces)?;
                    // even though we don't use POSSIBLE_TYPES_ALIAS, they're ok here

                    TypeDefinition::Interface(InterfaceType {
                        position: Pos::default(),
                        description,
                        name,
                        directives: vec![],
                        fields: fields.unwrap_or_else(|| vec![]),
                    })
                }
                TypeKind::Union => {
                    require_field_empty(FIELDS_ALIAS, fields)?;
                    require_field_empty(INPUT_FIELDS_ALIAS, input_fields)?;
                    require_field_empty(INTERFACES_ALIAS, interfaces)?;

                    TypeDefinition::Union(UnionType {
                        position: Pos::default(),
                        description,
                        name,
                        directives: vec![],
                        types: possible_types.unwrap_or_else(|| vec![]),
                    })
                }
                TypeKind::Enum => {
                    require_field_empty(FIELDS_ALIAS, fields)?;
                    require_field_empty(INPUT_FIELDS_ALIAS, input_fields)?;
                    require_field_empty(INTERFACES_ALIAS, interfaces)?;
                    require_field_empty(POSSIBLE_TYPES_ALIAS, possible_types)?;

                    TypeDefinition::Enum(EnumType {
                        position: Pos::default(),
                        description,
                        name,
                        directives: vec![],
                        values: enum_values.unwrap_or_else(|| vec![]),
                    })
                }
                TypeKind::InputObject => {
                    require_field_empty(FIELDS_ALIAS, fields)?;
                    require_field_empty(INTERFACES_ALIAS, interfaces)?;
                    require_field_empty(POSSIBLE_TYPES_ALIAS, possible_types)?;

                    TypeDefinition::InputObject(InputObjectType {
                        position: Pos::default(),
                        description,
                        name,
                        directives: vec![],
                        fields: input_fields.unwrap_or_else(|| vec![]),
                    })
                }
            };

            Ok(result)
        }
    }

    deserializer.deserialize_map(TypeDefinitionVisitor)
}

impl<'de> Deserialize<'de> for DeserializeWith<Field> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_field(deserializer).map(|value| DeserializeWith { value })
    }
}

fn deserialize_field<'de, D>(deserializer: D) -> Result<Field, D::Error>
where
    D: Deserializer<'de>,
{
    struct FieldVisitor;

    impl<'de> Visitor<'de> for FieldVisitor {
        type Value = Field;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("A Field object")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut name: Option<String> = None;
            let mut description: Option<String> = None;
            let mut value_type: Option<Type> = None;
            let mut input_fields: Option<Vec<InputValue>> = None;

            while let Some(key) = access.next_key()? {
                match key {
                    NAME_ALIAS => {
                        name = Some(access.next_value()?);
                    }
                    DESCRIPTION_ALIAS => {
                        description = access.next_value()?;
                    }
                    TYPE_ALIAS => {
                        value_type = DeserializeWith::deserialize_value(&mut access)?;
                    }
                    ARGS_ALIAS => {
                        input_fields = DeserializeWith::deserialize_array(&mut access)?;
                    }
                    IS_DEPRECATED_ALIAS => {
                        // not supported
                        access.next_value::<IgnoredAny>()?;
                    }
                    DEPRECATION_REASON_ALIAS => {
                        // not supported
                        access.next_value::<IgnoredAny>()?;
                    }
                    _ => handle_unexpected_key(key, &mut access)?,
                }
            }

            Ok(Field {
                position: Pos::default(),
                description,
                name: require_field(NAME_ALIAS, name)?,
                arguments: input_fields.unwrap_or_else(|| vec![]),
                field_type: require_field(TYPE_ALIAS, value_type)?,
                directives: vec![],
            })
        }
    }

    deserializer.deserialize_map(FieldVisitor)
}

impl<'de> Deserialize<'de> for DeserializeWith<InputValue> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_input_value(deserializer).map(|value| DeserializeWith { value })
    }
}

fn deserialize_input_value<'de, D>(deserializer: D) -> Result<InputValue, D::Error>
where
    D: Deserializer<'de>,
{
    struct InputValueVisitor;

    impl<'de> Visitor<'de> for InputValueVisitor {
        type Value = InputValue;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("A InputValue object")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut name: Option<String> = None;
            let mut description: Option<String> = None;
            let mut maybe_value_type: Option<Type> = None;
            let mut default_value_json: Option<json::Value> = None;

            while let Some(key) = access.next_key()? {
                match key {
                    NAME_ALIAS => {
                        name = Some(access.next_value()?);
                    }
                    DESCRIPTION_ALIAS => {
                        description = access.next_value()?;
                    }
                    TYPE_ALIAS => {
                        maybe_value_type = DeserializeWith::deserialize_value(&mut access)?;
                    }
                    DEFAULT_VALUE_ALIAS => {
                        default_value_json = access.next_value()?;
                    }
                    _ => {
                        println!(
                            "{:?}\n{:?}\n{:?}\n{:?}",
                            name, description, maybe_value_type, default_value_json
                        );

                        handle_unexpected_key(key, &mut access)?
                    }
                }
            }

            let value_type = require_field(TYPE_ALIAS, maybe_value_type)?;
            //let default_value = default_value_json.map(|v| json_value_to_graphql(&v, &value_type));

            Ok(InputValue {
                position: Pos::default(),
                description,
                name: require_field(NAME_ALIAS, name)?,
                value_type,
                default_value: None,
                directives: vec![],
            })
        }
    }

    deserializer.deserialize_map(InputValueVisitor)
}

impl<'de> Deserialize<'de> for DeserializeWith<Type> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_type_ref(deserializer).map(|value| DeserializeWith { value })
    }
}

fn deserialize_type_ref<'de, D>(deserializer: D) -> Result<Type, D::Error>
where
    D: Deserializer<'de>,
{
    struct TypeRefVisitor;

    impl<'de> Visitor<'de> for TypeRefVisitor {
        type Value = Type;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("A TypeRef object")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut kind: Option<String> = None;
            let mut name: Option<String> = None;
            let mut of_type: Option<Type> = None;

            while let Some(key) = access.next_key()? {
                match key {
                    KIND_ALIAS => {
                        kind = Some(access.next_value()?);
                    }
                    NAME_ALIAS => {
                        name = access.next_value()?;
                    }
                    OF_TYPE_ALIAS => {
                        of_type = DeserializeWith::deserialize_value(&mut access)?;
                    }
                    _ => handle_unexpected_key(key, &mut access)?,
                }
            }

            match require_field(KIND_ALIAS, kind)?.as_str() {
                "LIST" => {
                    require_field(OF_TYPE_ALIAS, of_type).map(|t| Type::ListType(Box::new(t)))
                }
                "NON_NULL" => {
                    require_field(OF_TYPE_ALIAS, of_type).map(|t| Type::NonNullType(Box::new(t)))
                }
                _ => require_field(NAME_ALIAS, name).map(Type::NamedType),
            }
        }
    }

    deserializer.deserialize_map(TypeRefVisitor)
}

impl<'de> Deserialize<'de> for DeserializeWith<NamedType> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_type_ref(deserializer).and_then(|type_ref| match type_ref {
            Type::NamedType(name) => Ok(DeserializeWith { value: name }),
            unexpected => Err(de::Error::custom(format_args!(
                "Expected NamedType, found {:?}",
                unexpected
            ))),
        })
    }
}

impl<'de> Deserialize<'de> for DeserializeWith<EnumValue> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_enum_value(deserializer).map(|value| DeserializeWith { value })
    }
}

fn deserialize_enum_value<'de, D>(deserializer: D) -> Result<EnumValue, D::Error>
where
    D: Deserializer<'de>,
{
    struct EnumValueVisitor;

    impl<'de> Visitor<'de> for EnumValueVisitor {
        type Value = EnumValue;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("An EnumValue object")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut name: Option<String> = None;
            let mut description: Option<String> = None;

            while let Some(key) = access.next_key()? {
                match key {
                    NAME_ALIAS => {
                        name = Some(access.next_value()?);
                    }
                    DESCRIPTION_ALIAS => {
                        description = access.next_value()?;
                    }
                    IS_DEPRECATED_ALIAS => {
                        // not supported
                        access.next_value::<IgnoredAny>()?;
                    }
                    DEPRECATION_REASON_ALIAS => {
                        // not supported
                        access.next_value::<IgnoredAny>()?;
                    }
                    _ => handle_unexpected_key(key, &mut access)?,
                }
            }

            Ok(EnumValue {
                position: Pos::default(),
                description,
                name: require_field(NAME_ALIAS, name)?,
                directives: vec![],
            })
        }
    }

    deserializer.deserialize_map(EnumValueVisitor)
}

struct DeserializeWith<T: Sized> {
    value: T,
}

impl<'de, T> DeserializeWith<T>
where
    DeserializeWith<T>: Deserialize<'de>,
{
    fn deserialize_value<M>(access: &mut M) -> Result<Option<T>, M::Error>
    where
        M: MapAccess<'de>,
    {
        access
            .next_value::<Option<DeserializeWith<T>>>()
            .map(|value| value.map(|v| v.value))
    }

    fn deserialize_array<M>(access: &mut M) -> Result<Option<Vec<T>>, M::Error>
    where
        M: MapAccess<'de>,
    {
        access
            .next_value::<Option<Vec<DeserializeWith<T>>>>()
            .map(|value| {
                value.map(|wrapped_fields| wrapped_fields.into_iter().map(|v| v.value).collect())
            })
    }
}

#[derive(Deserialize)]
enum TypeKind {
    #[serde(rename(deserialize = "SCALAR"))]
    Scalar,
    #[serde(rename(deserialize = "OBJECT"))]
    Object,
    #[serde(rename(deserialize = "INTERFACE"))]
    Interface,
    #[serde(rename(deserialize = "UNION"))]
    Union,
    #[serde(rename(deserialize = "ENUM"))]
    Enum,
    #[serde(rename(deserialize = "INPUT_OBJECT"))]
    InputObject,
}

fn require_field<T, E>(key: &'static str, field: Option<T>) -> Result<T, E>
where
    E: de::Error,
{
    field.ok_or_else(|| de::Error::missing_field(key))
}

fn require_field_empty<T, E>(key: &'static str, field: Option<T>) -> Result<(), E>
where
    E: de::Error,
{
    if field.is_none() {
        Ok(())
    } else {
        Err(de::Error::unknown_field(key, &["not this field"]))
    }
}

fn handle_unexpected_key<'de, M>(key: &str, access: &mut M) -> Result<(), M::Error>
where
    M: MapAccess<'de>,
{
    log::debug!("Unknown/unsupported key '{}'", key);

    // ignore our next entry
    access.next_value::<IgnoredAny>()?;

    Ok(())
}
