//! GraphQL API for Kotoba Schema Management
//!
//! This module provides a GraphQL interface for managing graph schemas,
//! including CRUD operations and validation.

use async_graphql::*;
use kotoba_schema::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;

/// GraphQL context for schema operations
pub struct SchemaContext {
    pub schema_manager: Arc<SchemaManager>,
}

impl SchemaContext {
    pub fn new(schema_manager: Arc<SchemaManager>) -> Self {
        Self { schema_manager }
    }
}

/// GraphQL schema definition
pub type KotobaSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Query root for GraphQL schema
#[derive(Default)]
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get all registered schemas
    async fn schemas(&self, ctx: &Context<'_>) -> Result<Vec<GraphSchemaGQL>> {
        let context = ctx.data::<SchemaContext>()?;
        let schema_ids = context.schema_manager.list_schemas()?;

        let mut schemas = Vec::new();
        for schema_id in schema_ids {
            if let Ok(Some(schema)) = context.schema_manager.get_schema(&schema_id) {
                schemas.push(schema.into());
            }
        }

        Ok(schemas)
    }

    /// Get a specific schema by ID
    async fn schema(&self, ctx: &Context<'_>, id: String) -> Result<Option<GraphSchemaGQL>> {
        let context = ctx.data::<SchemaContext>()?;
        match context.schema_manager.get_schema(&id)? {
            Some(schema) => Ok(Some(schema.into())),
            None => Ok(None),
        }
    }

    /// Get schema statistics
    async fn schema_stats(&self, ctx: &Context<'_>, id: String) -> Result<Option<SchemaStatisticsGQL>> {
        let context = ctx.data::<SchemaContext>()?;
        match context.schema_manager.get_schema_statistics(&id)? {
            Some(stats) => Ok(Some(stats.into())),
            None => Ok(None),
        }
    }

    /// Validate graph data against a schema
    async fn validate_graph(
        &self,
        ctx: &Context<'_>,
        schema_id: String,
        graph_data: String,
    ) -> Result<ValidationResultGQL> {
        let context = ctx.data::<SchemaContext>()?;

        // Parse JSON graph data
        let graph_json: serde_json::Value = serde_json::from_str(&graph_data)
            .map_err(|e| Error::new(format!("Invalid JSON: {}", e)))?;

        let result = context.schema_manager.validate_graph_data(&schema_id, &graph_json)?;
        Ok(result.into())
    }

    /// List all available schemas
    async fn schema_list(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let context = ctx.data::<SchemaContext>()?;
        context.schema_manager.list_schemas()
            .map_err(|e| Error::new(format!("Failed to list schemas: {}", e)))
    }

    /// Get schema metadata
    async fn schema_metadata(&self, ctx: &Context<'_>, id: String) -> Result<Option<HashMap<String, ValueGQL>>> {
        let context = ctx.data::<SchemaContext>()?;
        match context.schema_manager.get_schema_metadata(&id)? {
            Some(metadata) => {
                let gql_metadata: HashMap<String, ValueGQL> = metadata.into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect();
                Ok(Some(gql_metadata))
            },
            None => Ok(None),
        }
    }

    /// Health check for the schema system
    async fn schema_health(&self) -> Result<String> {
        // Simple health check
        Ok("Schema system is healthy".to_string())
    }
}

/// Mutation root for GraphQL schema
#[derive(Default)]
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a new schema
    async fn create_schema(
        &self,
        ctx: &Context<'_>,
        id: String,
        name: String,
        version: String,
        description: Option<String>,
    ) -> Result<GraphSchemaGQL> {
        let context = ctx.data::<SchemaContext>()?;

        let schema = GraphSchema::new(id, name, version);
        let mut schema_with_desc = schema;
        if let Some(desc) = description {
            schema_with_desc.description = Some(desc);
        }

        context.schema_manager.register_schema(schema_with_desc.clone())?;
        Ok(schema_with_desc.into())
    }

    /// Update an existing schema
    async fn update_schema(
        &self,
        ctx: &Context<'_>,
        id: String,
        name: Option<String>,
        version: Option<String>,
        description: Option<String>,
    ) -> Result<GraphSchemaGQL> {
        let context = ctx.data::<SchemaContext>()?;

        // Get existing schema
        let mut existing_schema = context.schema_manager.get_schema(&id)?
            .ok_or_else(|| Error::new(format!("Schema '{}' not found", id)))?;

        // Update fields
        if let Some(name) = name {
            existing_schema.name = name;
        }
        if let Some(version) = version {
            existing_schema.version = version;
        }
        if let Some(description) = description {
            existing_schema.description = Some(description);
        }

        context.schema_manager.update_schema(existing_schema.clone())?;
        Ok(existing_schema.into())
    }

    /// Delete a schema
    async fn delete_schema(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let context = ctx.data::<SchemaContext>()?;
        context.schema_manager.delete_schema(&id)?;
        Ok(true)
    }

    /// Add a vertex type to a schema
    async fn add_vertex_type(
        &self,
        ctx: &Context<'_>,
        schema_id: String,
        vertex_type: VertexTypeInput,
    ) -> Result<GraphSchemaGQL> {
        let context = ctx.data::<SchemaContext>()?;

        let mut schema = context.schema_manager.get_schema(&schema_id)?
            .ok_or_else(|| Error::new(format!("Schema '{}' not found", schema_id)))?;

        let vertex_schema: VertexTypeSchema = vertex_type.into();
        schema.add_vertex_type(vertex_schema);

        context.schema_manager.update_schema(schema.clone())?;
        Ok(schema.into())
    }

    /// Add an edge type to a schema
    async fn add_edge_type(
        &self,
        ctx: &Context<'_>,
        schema_id: String,
        edge_type: EdgeTypeInput,
    ) -> Result<GraphSchemaGQL> {
        let context = ctx.data::<SchemaContext>()?;

        let mut schema = context.schema_manager.get_schema(&schema_id)?
            .ok_or_else(|| Error::new(format!("Schema '{}' not found", schema_id)))?;

        let edge_schema: EdgeTypeSchema = edge_type.into();
        schema.add_edge_type(edge_schema);

        context.schema_manager.update_schema(schema.clone())?;
        Ok(schema.into())
    }

    /// Clone a schema
    async fn clone_schema(
        &self,
        ctx: &Context<'_>,
        source_id: String,
        target_id: String,
    ) -> Result<GraphSchemaGQL> {
        let context = ctx.data::<SchemaContext>()?;
        context.schema_manager.clone_schema(&source_id, &target_id)?;

        let cloned_schema = context.schema_manager.get_schema(&target_id)?
            .ok_or_else(|| Error::new("Failed to retrieve cloned schema".to_string()))?;

        Ok(cloned_schema.into())
    }

    /// Update schema metadata
    async fn update_schema_metadata(
        &self,
        ctx: &Context<'_>,
        schema_id: String,
        metadata: HashMap<String, ValueGQL>,
    ) -> Result<bool> {
        let context = ctx.data::<SchemaContext>()?;

        let gql_metadata: HashMap<String, Value> = metadata.into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();

        context.schema_manager.update_schema_metadata(&schema_id, gql_metadata)?;
        Ok(true)
    }

    /// Validate and apply schema changes
    async fn validate_schema_changes(
        &self,
        ctx: &Context<'_>,
        schema_id: String,
        changes: Vec<SchemaChange>,
    ) -> Result<ValidationResultGQL> {
        let context = ctx.data::<SchemaContext>()?;

        // Get current schema
        let mut schema = context.schema_manager.get_schema(&schema_id)?
            .ok_or_else(|| Error::new(format!("Schema '{}' not found", schema_id)))?;

        // Apply changes
        for change in changes {
            match change {
                SchemaChange::AddVertexType(vertex_input) => {
                    let vertex_schema: VertexTypeSchema = vertex_input.into();
                    schema.add_vertex_type(vertex_schema);
                },
                SchemaChange::AddEdgeType(edge_input) => {
                    let edge_schema: EdgeTypeSchema = edge_input.into();
                    schema.add_edge_type(edge_schema);
                },
                SchemaChange::UpdateName(new_name) => {
                    schema.name = new_name;
                },
                SchemaChange::UpdateVersion(new_version) => {
                    schema.version = new_version;
                },
                SchemaChange::UpdateDescription(new_desc) => {
                    schema.description = Some(new_desc);
                },
            }
        }

        // Validate the updated schema
        let validation = schema.validate_schema();

        // If valid, save the changes
        if validation.is_valid {
            context.schema_manager.update_schema(schema)?;
        }

        Ok(validation.into())
    }
}

// GraphQL type definitions

/// GraphQL representation of GraphSchema
#[derive(SimpleObject)]
pub struct GraphSchemaGQL {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub vertex_types: Vec<VertexTypeGQL>,
    pub edge_types: Vec<EdgeTypeGQL>,
    pub constraints: Vec<String>, // Simplified representation
}

/// GraphQL representation of VertexTypeSchema
#[derive(SimpleObject)]
pub struct VertexTypeGQL {
    pub name: String,
    pub description: Option<String>,
    pub required_properties: Vec<String>,
    pub properties: Vec<PropertyGQL>,
    pub inherits: Vec<String>,
}

/// GraphQL representation of EdgeTypeSchema
#[derive(SimpleObject)]
pub struct EdgeTypeGQL {
    pub name: String,
    pub description: Option<String>,
    pub source_types: Vec<String>,
    pub target_types: Vec<String>,
    pub required_properties: Vec<String>,
    pub properties: Vec<PropertyGQL>,
    pub directed: bool,
}

/// GraphQL representation of PropertySchema
#[derive(SimpleObject)]
pub struct PropertyGQL {
    pub name: String,
    pub property_type: String,
    pub description: Option<String>,
    pub required: bool,
}

/// GraphQL representation of SchemaStatistics
#[derive(SimpleObject)]
pub struct SchemaStatisticsGQL {
    pub vertex_types: usize,
    pub edge_types: usize,
    pub constraints: usize,
    pub total_properties: usize,
}

/// GraphQL representation of ValidationResult
#[derive(SimpleObject)]
pub struct ValidationResultGQL {
    pub is_valid: bool,
    pub errors: Vec<ValidationErrorGQL>,
    pub warnings: Vec<String>,
}

/// GraphQL representation of ValidationError
#[derive(SimpleObject)]
pub struct ValidationErrorGQL {
    pub error_type: String,
    pub message: String,
    pub element_id: Option<String>,
    pub property: Option<String>,
}

/// GraphQL representation of Value
#[derive(InputObject, SimpleObject)]
#[graphql(input_name = "ValueInput")]
pub struct ValueGQL {
    #[graphql(flatten)]
    pub value: ValueTypeGQL,
}

/// GraphQL representation of different value types
#[derive(InputObject, SimpleObject)]
#[graphql(input_name = "ValueTypeInput")]
pub struct ValueTypeGQL {
    pub string_value: Option<String>,
    pub int_value: Option<i64>,
    pub float_value: Option<f64>,
    pub bool_value: Option<bool>,
    pub array_value: Option<Vec<ValueGQL>>,
    pub object_value: Option<HashMap<String, ValueGQL>>,
}

// Input types for mutations

/// Input for vertex type creation
#[derive(InputObject)]
pub struct VertexTypeInput {
    pub name: String,
    pub description: Option<String>,
    pub required_properties: Vec<String>,
    pub properties: Vec<PropertyInput>,
    pub inherits: Vec<String>,
}

/// Input for edge type creation
#[derive(InputObject)]
pub struct EdgeTypeInput {
    pub name: String,
    pub description: Option<String>,
    pub source_types: Vec<String>,
    pub target_types: Vec<String>,
    pub required_properties: Vec<String>,
    pub properties: Vec<PropertyInput>,
    pub directed: bool,
}

/// Input for property creation
#[derive(InputObject)]
pub struct PropertyInput {
    pub name: String,
    pub property_type: String,
    pub description: Option<String>,
    pub required: bool,
    pub constraints: Vec<PropertyConstraintInput>,
}

/// Input for property constraints
#[derive(InputObject)]
pub struct PropertyConstraintInput {
    pub constraint_type: String,
    pub value: Option<String>,
}

/// Schema change operations for batch updates
#[derive(InputObject)]
pub enum SchemaChange {
    AddVertexType(VertexTypeInput),
    AddEdgeType(EdgeTypeInput),
    UpdateName(String),
    UpdateVersion(String),
    UpdateDescription(String),
}

// Conversion implementations

impl From<GraphSchema> for GraphSchemaGQL {
    fn from(schema: GraphSchema) -> Self {
        GraphSchemaGQL {
            id: schema.id,
            name: schema.name,
            description: schema.description,
            version: schema.version,
            vertex_types: schema.vertex_types.into_values().map(Into::into).collect(),
            edge_types: schema.edge_types.into_values().map(Into::into).collect(),
            constraints: schema.constraints.iter().map(|c| format!("{:?}", c)).collect(),
        }
    }
}

impl From<VertexTypeSchema> for VertexTypeGQL {
    fn from(vertex: VertexTypeSchema) -> Self {
        VertexTypeGQL {
            name: vertex.name,
            description: vertex.description,
            required_properties: vertex.required_properties,
            properties: vertex.properties.into_values().map(Into::into).collect(),
            inherits: vertex.inherits,
        }
    }
}

impl From<EdgeTypeSchema> for EdgeTypeGQL {
    fn from(edge: EdgeTypeSchema) -> Self {
        EdgeTypeGQL {
            name: edge.name,
            description: edge.description,
            source_types: edge.source_types,
            target_types: edge.target_types,
            required_properties: edge.required_properties,
            properties: edge.properties.into_values().map(Into::into).collect(),
            directed: edge.directed,
        }
    }
}

impl From<PropertySchema> for PropertyGQL {
    fn from(prop: PropertySchema) -> Self {
        PropertyGQL {
            name: prop.name,
            property_type: format!("{:?}", prop.property_type),
            description: prop.description,
            required: prop.required,
        }
    }
}

impl From<SchemaStatistics> for SchemaStatisticsGQL {
    fn from(stats: SchemaStatistics) -> Self {
        SchemaStatisticsGQL {
            vertex_types: stats.vertex_types,
            edge_types: stats.edge_types,
            constraints: stats.constraints,
            total_properties: stats.total_properties,
        }
    }
}

impl From<kotoba_schema::ValidationResult> for ValidationResultGQL {
    fn from(result: kotoba_schema::ValidationResult) -> Self {
        ValidationResultGQL {
            is_valid: result.is_valid,
            errors: result.errors.into_iter().map(Into::into).collect(),
            warnings: result.warnings,
        }
    }
}

impl From<ValidationError> for ValidationErrorGQL {
    fn from(error: ValidationError) -> Self {
        ValidationErrorGQL {
            error_type: format!("{:?}", error.error_type),
            message: error.message,
            element_id: error.element_id,
            property: error.property,
        }
    }
}

impl From<Value> for ValueGQL {
    fn from(value: Value) -> Self {
        match value {
            Value::String(s) => ValueGQL {
                value: ValueTypeGQL {
                    string_value: Some(s),
                    int_value: None,
                    float_value: None,
                    bool_value: None,
                    array_value: None,
                    object_value: None,
                }
            },
            Value::Integer(i) => ValueGQL {
                value: ValueTypeGQL {
                    string_value: None,
                    int_value: Some(i),
                    float_value: None,
                    bool_value: None,
                    array_value: None,
                    object_value: None,
                }
            },
            Value::Float(f) => ValueGQL {
                value: ValueTypeGQL {
                    string_value: None,
                    int_value: None,
                    float_value: Some(f),
                    bool_value: None,
                    array_value: None,
                    object_value: None,
                }
            },
            Value::Boolean(b) => ValueGQL {
                value: ValueTypeGQL {
                    string_value: None,
                    int_value: None,
                    float_value: None,
                    bool_value: Some(b),
                    array_value: None,
                    object_value: None,
                }
            },
            Value::Array(arr) => ValueGQL {
                value: ValueTypeGQL {
                    string_value: None,
                    int_value: None,
                    float_value: None,
                    bool_value: None,
                    array_value: Some(arr.into_iter().map(Into::into).collect()),
                    object_value: None,
                }
            },
            Value::Object(obj) => ValueGQL {
                value: ValueTypeGQL {
                    string_value: None,
                    int_value: None,
                    float_value: None,
                    bool_value: None,
                    array_value: None,
                    object_value: Some(obj.into_iter().map(|(k, v)| (k, v.into())).collect()),
                }
            },
        }
    }
}

impl From<ValueGQL> for Value {
    fn from(value: ValueGQL) -> Self {
        if let Some(s) = value.value.string_value {
            Value::String(s)
        } else if let Some(i) = value.value.int_value {
            Value::Integer(i)
        } else if let Some(f) = value.value.float_value {
            Value::Float(f)
        } else if let Some(b) = value.value.bool_value {
            Value::Boolean(b)
        } else if let Some(arr) = value.value.array_value {
            Value::Array(arr.into_iter().map(Into::into).collect())
        } else if let Some(obj) = value.value.object_value {
            Value::Object(obj.into_iter().map(|(k, v)| (k, v.into())).collect())
        } else {
            Value::String("".to_string()) // Default fallback
        }
    }
}

impl From<VertexTypeInput> for VertexTypeSchema {
    fn from(input: VertexTypeInput) -> Self {
        let properties: HashMap<String, PropertySchema> = input.properties
            .into_iter()
            .map(|p| {
                let property_type = match p.property_type.as_str() {
                    "String" => PropertyType::String,
                    "Integer" => PropertyType::Integer,
                    "Float" => PropertyType::Float,
                    "Boolean" => PropertyType::Boolean,
                    "DateTime" => PropertyType::DateTime,
                    "Json" => PropertyType::Json,
                    _ => PropertyType::String, // Default fallback
                };

                let property_schema = PropertySchema {
                    name: p.name,
                    property_type,
                    description: p.description,
                    required: p.required,
                    default_value: None,
                    constraints: vec![], // TODO: Convert from input
                };

                (p.name, property_schema)
            })
            .collect();

        VertexTypeSchema {
            name: input.name,
            description: input.description,
            required_properties: input.required_properties,
            properties,
            inherits: input.inherits,
            constraints: vec![],
        }
    }
}

impl From<EdgeTypeInput> for EdgeTypeSchema {
    fn from(input: EdgeTypeInput) -> Self {
        let properties: HashMap<String, PropertySchema> = input.properties
            .into_iter()
            .map(|p| {
                let property_type = match p.property_type.as_str() {
                    "String" => PropertyType::String,
                    "Integer" => PropertyType::Integer,
                    "Float" => PropertyType::Float,
                    "Boolean" => PropertyType::Boolean,
                    "DateTime" => PropertyType::DateTime,
                    "Json" => PropertyType::Json,
                    _ => PropertyType::String, // Default fallback
                };

                let property_schema = PropertySchema {
                    name: p.name,
                    property_type,
                    description: p.description,
                    required: p.required,
                    default_value: None,
                    constraints: vec![], // TODO: Convert from input
                };

                (p.name, property_schema)
            })
            .collect();

        EdgeTypeSchema {
            name: input.name,
            description: input.description,
            source_types: input.source_types,
            target_types: input.target_types,
            required_properties: input.required_properties,
            properties,
            directed: input.directed,
            constraints: vec![],
        }
    }
}

/// Create GraphQL schema
pub fn create_schema(schema_manager: Arc<SchemaManager>) -> KotobaSchema {
    let context = SchemaContext::new(schema_manager);

    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(context)
        .finish()
}

/// GraphQL handler for HTTP requests
pub async fn graphql_handler(
    schema: &KotobaSchema,
    request_body: String,
) -> Result<String> {
    let request: async_graphql::Request = serde_json::from_str(&request_body)
        .map_err(|e| Error::new(format!("Invalid GraphQL request: {}", e)))?;

    let response = schema.execute(request).await;
    let json_response = serde_json::to_string(&response)
        .map_err(|e| Error::new(format!("Failed to serialize response: {}", e)))?;

    Ok(json_response)
}
