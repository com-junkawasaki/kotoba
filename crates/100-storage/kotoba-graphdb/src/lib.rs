//! `kotoba-graphdb`
//!
//! RocksDB-based Graph Database for KotobaDB.
//! Provides efficient storage and querying of graph data structures.

use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use rocksdb::{DB, ColumnFamilyDescriptor, Options, WriteBatch, IteratorMode};
use serde::{Deserialize, Serialize};
use tracing::{info, error, instrument};
use dashmap::DashMap;
use bincode;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Main GraphDB instance
pub struct GraphDB {
    /// RocksDB instance
    db: Arc<DB>,
    /// Node cache
    node_cache: Arc<DashMap<String, Node>>,
    /// Edge cache
    edge_cache: Arc<DashMap<String, Edge>>,
    /// Schema information
    schema: Arc<RwLock<Schema>>,
}

/// Node (Vertex) in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique node ID
    pub id: String,
    /// Node labels
    pub labels: Vec<String>,
    /// Node properties
    pub properties: BTreeMap<String, PropertyValue>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Edge (Relationship) in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Unique edge ID
    pub id: String,
    /// Source node ID
    pub from_node: String,
    /// Target node ID
    pub to_node: String,
    /// Edge label
    pub label: String,
    /// Edge properties
    pub properties: BTreeMap<String, PropertyValue>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Property value types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PropertyValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// Date/time value
    Date(DateTime<Utc>),
    /// List of values
    List(Vec<PropertyValue>),
    /// Map of values
    Map(BTreeMap<String, PropertyValue>),
}

/// Graph query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    /// Node patterns to match
    pub node_patterns: Vec<NodePattern>,
    /// Edge patterns to match
    pub edge_patterns: Vec<EdgePattern>,
    /// WHERE conditions
    pub conditions: Vec<QueryCondition>,
    /// RETURN specifications
    pub returns: Vec<ReturnSpec>,
    /// LIMIT clause
    pub limit: Option<usize>,
    /// SKIP clause
    pub skip: Option<usize>,
}

/// Node pattern for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePattern {
    /// Variable name for the node
    pub variable: Option<String>,
    /// Node labels to match
    pub labels: Vec<String>,
    /// Property conditions
    pub properties: BTreeMap<String, PropertyCondition>,
}

/// Edge pattern for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgePattern {
    /// Variable name for the edge
    pub variable: Option<String>,
    /// Edge label to match
    pub label: Option<String>,
    /// Property conditions
    pub properties: BTreeMap<String, PropertyCondition>,
    /// Source node variable
    pub from_variable: Option<String>,
    /// Target node variable
    pub to_variable: Option<String>,
}

/// Property condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyCondition {
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Value to compare against
    pub value: PropertyValue,
}

/// Comparison operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    Contains,
    StartsWith,
    EndsWith,
    In,
}

/// Query condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryCondition {
    /// Property comparison
    Property {
        variable: String,
        property: String,
        condition: PropertyCondition,
    },
    /// Logical AND
    And(Box<QueryCondition>, Box<QueryCondition>),
    /// Logical OR
    Or(Box<QueryCondition>, Box<QueryCondition>),
    /// Logical NOT
    Not(Box<QueryCondition>),
}

/// Return specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReturnSpec {
    /// Return node
    Node(String),
    /// Return edge
    Edge(String),
    /// Return property
    Property { variable: String, property: String },
    /// Return count
    Count,
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Column names
    pub columns: Vec<String>,
    /// Result rows
    pub rows: Vec<ResultRow>,
    /// Query statistics
    pub statistics: QueryStatistics,
}

/// Result row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultRow {
    /// Row data
    pub data: BTreeMap<String, PropertyValue>,
}

/// Query statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStatistics {
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Number of nodes scanned
    pub nodes_scanned: u64,
    /// Number of edges scanned
    pub edges_scanned: u64,
    /// Number of results returned
    pub results_returned: u64,
}

/// Graph transaction
pub struct GraphTransaction<'a> {
    /// GraphDB reference
    db: &'a GraphDB,
    /// Write batch for atomic operations
    batch: WriteBatch,
    /// Modified nodes
    modified_nodes: HashSet<String>,
    /// Modified edges
    modified_edges: HashSet<String>,
}

/// Database schema information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Schema {
    /// Node labels and their properties
    node_labels: HashMap<String, LabelSchema>,
    /// Edge labels and their properties
    edge_labels: HashMap<String, LabelSchema>,
    /// Property indexes
    indexes: HashMap<String, IndexSchema>,
}

/// Label schema
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LabelSchema {
    /// Label name
    name: String,
    /// Property definitions
    properties: HashMap<String, PropertySchema>,
}

/// Property schema
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PropertySchema {
    /// Property name
    name: String,
    /// Property type
    data_type: PropertyType,
}

/// Property types
#[derive(Debug, Clone, Serialize, Deserialize)]
enum PropertyType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    List,
    Map,
}

/// Index schema
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexSchema {
    /// Index name
    name: String,
    /// Indexed properties
    properties: Vec<String>,
    /// Index type
    index_type: IndexType,
}

/// Index types
#[derive(Debug, Clone, Serialize, Deserialize)]
enum IndexType {
    /// Single property index
    Single,
    /// Composite property index
    Composite,
    /// Full-text search index
    FullText,
}

impl GraphDB {
    /// Create a new GraphDB instance
    pub async fn new(path: &str) -> Result<Self, GraphError> {
        info!("Initializing RocksDB GraphDB at: {}", path);

        // Configure RocksDB options
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        db_opts.set_max_background_jobs(4);
        db_opts.set_compaction_style(rocksdb::DBCompactionStyle::Level);
        db_opts.set_target_file_size_base(64 * 1024 * 1024); // 64MB
        db_opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        // Define column families
        let cf_names = vec![
            "default",
            "nodes",
            "edges",
            "indexes",
            "schema",
            "metadata",
        ];

        let cf_descriptors: Vec<ColumnFamilyDescriptor> = cf_names
            .iter()
            .map(|name| {
                let mut cf_opts = Options::default();
                cf_opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
                ColumnFamilyDescriptor::new(*name, cf_opts)
            })
            .collect();

        // Open database
        let db = DB::open_cf_descriptors(&db_opts, path, cf_descriptors)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        let graphdb = Self {
            db: Arc::new(db),
            node_cache: Arc::new(DashMap::new()),
            edge_cache: Arc::new(DashMap::new()),
            schema: Arc::new(RwLock::new(Schema::default())),
        };

        // Load schema
        graphdb.load_schema().await?;

        info!("RocksDB GraphDB initialized successfully");
        Ok(graphdb)
    }

    /// Create a new node
    #[instrument(skip(self))]
    pub async fn create_node(
        &self,
        id: Option<String>,
        labels: Vec<String>,
        properties: BTreeMap<String, PropertyValue>,
    ) -> Result<String, GraphError> {
        let node_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now();

        let node = Node {
            id: node_id.clone(),
            labels: labels.clone(),
            properties: properties.clone(),
            created_at: now,
            updated_at: now,
        };

        // Serialize node
        let node_data = bincode::serialize(&node)
            .map_err(|e| GraphError::SerializationError(e.to_string()))?;

        // Store in nodes column family
        let cf_nodes = self.db.cf_handle("nodes")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("nodes".to_string()))?;

        let node_key = format!("node:{}", node_id);
        self.db.put_cf(&cf_nodes, &node_key, node_data)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        // Update schema
        self.update_schema_for_node(&node).await?;

        // Create indexes
        self.create_indexes_for_node(&node).await?;

        // Cache the node
        self.node_cache.insert(node_id.clone(), node);

        info!("Created node: {}", node_id);
        Ok(node_id)
    }

    /// Get a node by ID
    #[instrument(skip(self))]
    pub async fn get_node(&self, node_id: &str) -> Result<Option<Node>, GraphError> {
        // Check cache first
        if let Some(node) = self.node_cache.get(node_id) {
            return Ok(Some(node.clone()));
        }

        // Load from database
        let cf_nodes = self.db.cf_handle("nodes")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("nodes".to_string()))?;

        let node_key = format!("node:{}", node_id);

        if let Some(node_data) = self.db.get_cf(&cf_nodes, &node_key)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))? {

            let node: Node = bincode::deserialize(&node_data)
                .map_err(|e| GraphError::SerializationError(e.to_string()))?;

            // Cache the node
            self.node_cache.insert(node_id.to_string(), node.clone());

            Ok(Some(node))
        } else {
            Ok(None)
        }
    }

    /// Update a node
    #[instrument(skip(self))]
    pub async fn update_node(
        &self,
        node_id: &str,
        properties: BTreeMap<String, PropertyValue>,
    ) -> Result<(), GraphError> {
        let mut node = self.get_node(node_id).await?
            .ok_or_else(|| GraphError::NodeNotFound(node_id.to_string()))?;

        node.properties.extend(properties);
        node.updated_at = Utc::now();

        // Serialize updated node
        let node_data = bincode::serialize(&node)
            .map_err(|e| GraphError::SerializationError(e.to_string()))?;

        // Store updated node
        let cf_nodes = self.db.cf_handle("nodes")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("nodes".to_string()))?;

        let node_key = format!("node:{}", node_id);
        self.db.put_cf(&cf_nodes, &node_key, node_data)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        // Update indexes
        self.update_indexes_for_node(&node).await?;

        // Update cache
        self.node_cache.insert(node_id.to_string(), node);

        info!("Updated node: {}", node_id);
        Ok(())
    }

    /// Delete a node
    #[instrument(skip(self))]
    pub async fn delete_node(&self, node_id: &str) -> Result<(), GraphError> {
        // Delete from database
        let cf_nodes = self.db.cf_handle("nodes")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("nodes".to_string()))?;

        let node_key = format!("node:{}", node_id);
        self.db.delete_cf(&cf_nodes, &node_key)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        // Delete related edges
        self.delete_edges_for_node(node_id).await?;

        // Delete from indexes
        self.delete_indexes_for_node(node_id).await?;

        // Remove from cache
        self.node_cache.remove(node_id);

        info!("Deleted node: {}", node_id);
        Ok(())
    }

    /// Create an edge
    #[instrument(skip(self))]
    pub async fn create_edge(
        &self,
        id: Option<String>,
        from_node: &str,
        to_node: &str,
        label: String,
        properties: BTreeMap<String, PropertyValue>,
    ) -> Result<String, GraphError> {
        let edge_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now();

        let edge = Edge {
            id: edge_id.clone(),
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            label: label.clone(),
            properties: properties.clone(),
            created_at: now,
            updated_at: now,
        };

        // Serialize edge
        let edge_data = bincode::serialize(&edge)
            .map_err(|e| GraphError::SerializationError(e.to_string()))?;

        // Store in edges column family
        let cf_edges = self.db.cf_handle("edges")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("edges".to_string()))?;

        let edge_key = format!("edge:{}", edge_id);
        self.db.put_cf(&cf_edges, &edge_key, edge_data)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        // Store reverse mappings for efficient traversal
        self.store_edge_mappings(&edge).await?;

        // Update schema
        self.update_schema_for_edge(&edge).await?;

        // Create indexes
        self.create_indexes_for_edge(&edge).await?;

        // Cache the edge
        self.edge_cache.insert(edge_id.clone(), edge);

        info!("Created edge: {} ({} -> {} : {})", edge_id, from_node, to_node, label);
        Ok(edge_id)
    }

    /// Get an edge by ID
    #[instrument(skip(self))]
    pub async fn get_edge(&self, edge_id: &str) -> Result<Option<Edge>, GraphError> {
        // Check cache first
        if let Some(edge) = self.edge_cache.get(edge_id) {
            return Ok(Some(edge.clone()));
        }

        // Load from database
        let cf_edges = self.db.cf_handle("edges")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("edges".to_string()))?;

        let edge_key = format!("edge:{}", edge_id);

        if let Some(edge_data) = self.db.get_cf(&cf_edges, &edge_key)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))? {

            let edge: Edge = bincode::deserialize(&edge_data)
                .map_err(|e| GraphError::SerializationError(e.to_string()))?;

            // Cache the edge
            self.edge_cache.insert(edge_id.to_string(), edge.clone());

            Ok(Some(edge))
        } else {
            Ok(None)
        }
    }

    /// Get edges from a node
    #[instrument(skip(self))]
    pub async fn get_edges_from_node(&self, node_id: &str, label: Option<&str>) -> Result<Vec<Edge>, GraphError> {
        let cf_edges = self.db.cf_handle("edges")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("edges".to_string()))?;

        let prefix = format!("outgoing:{}:", node_id);
        let mut edges = Vec::new();

        let iter = self.db.iterator_cf(&cf_edges, IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));
        for item in iter {
            let (key, value) = item.map_err(|e| GraphError::RocksDBError(e.to_string()))?;
            if key.starts_with(prefix.as_bytes()) {
                let edge_id = String::from_utf8(value.to_vec())
                    .map_err(|_| GraphError::InvalidData("Invalid edge ID".to_string()))?;

                if let Some(edge) = self.get_edge(&edge_id).await? {
                    if label.is_none() || edge.label == label.unwrap() {
                        edges.push(edge);
                    }
                }
            }
        }

        Ok(edges)
    }

    /// Get edges to a node
    #[instrument(skip(self))]
    pub async fn get_edges_to_node(&self, node_id: &str, label: Option<&str>) -> Result<Vec<Edge>, GraphError> {
        let cf_edges = self.db.cf_handle("edges")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("edges".to_string()))?;

        let prefix = format!("incoming:{}:", node_id);
        let mut edges = Vec::new();

        let iter = self.db.iterator_cf(&cf_edges, IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));
        for item in iter {
            let (key, value) = item.map_err(|e| GraphError::RocksDBError(e.to_string()))?;
            if key.starts_with(prefix.as_bytes()) {
                let edge_id = String::from_utf8(value.to_vec())
                    .map_err(|_| GraphError::InvalidData("Invalid edge ID".to_string()))?;

                if let Some(edge) = self.get_edge(&edge_id).await? {
                    if label.is_none() || edge.label == label.unwrap() {
                        edges.push(edge);
                    }
                }
            }
        }

        Ok(edges)
    }

    /// Execute a graph query
    #[instrument(skip(self, query))]
    pub async fn execute_query(&self, query: GraphQuery) -> Result<QueryResult, GraphError> {
        let start_time = std::time::Instant::now();

        // This is a simplified implementation
        // In a full implementation, this would parse the query and execute it efficiently
        let mut results = Vec::new();
        let mut nodes_scanned = 0u64;
        let edges_scanned = 0u64;

        // For now, return all nodes that match the first node pattern
        if let Some(node_pattern) = query.node_patterns.first() {
            let nodes = self.query_nodes_by_pattern(node_pattern).await?;
            nodes_scanned = nodes.len() as u64;

            for node in nodes {
                let mut row_data = BTreeMap::new();
                if let Some(var) = &node_pattern.variable {
                    row_data.insert(var.clone(), PropertyValue::String(node.id.clone()));
                }
                results.push(ResultRow { data: row_data });
            }
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        let statistics = QueryStatistics {
            execution_time_ms: execution_time,
            nodes_scanned,
            edges_scanned,
            results_returned: results.len() as u64,
        };

        Ok(QueryResult {
            columns: query.returns.iter().map(|r| format!("{:?}", r)).collect(),
            rows: results,
            statistics,
        })
    }

    /// Start a transaction
    pub async fn begin_transaction(&self) -> GraphTransaction {
        GraphTransaction {
            db: self,
            batch: WriteBatch::default(),
            modified_nodes: HashSet::new(),
            modified_edges: HashSet::new(),
        }
    }

    /// Scan all nodes
    #[instrument(skip(self))]
    pub async fn scan_nodes(&self) -> Result<Vec<Node>, GraphError> {
        let cf_nodes = self.db.cf_handle("nodes")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("nodes".to_string()))?;

        let mut nodes = Vec::new();
        let iter = self.db.iterator_cf(&cf_nodes, IteratorMode::Start);

        for item in iter {
            let (key, value) = item.map_err(|e| GraphError::RocksDBError(e.to_string()))?;
            let _node_id = String::from_utf8(key.to_vec())
                .map_err(|e| GraphError::SerializationError(e.to_string()))?;
            let node: Node = bincode::deserialize(&value)
                .map_err(|e| GraphError::SerializationError(e.to_string()))?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    /// Scan all edges
    #[instrument(skip(self))]
    pub async fn scan_edges(&self) -> Result<Vec<Edge>, GraphError> {
        let cf_edges = self.db.cf_handle("edges")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("edges".to_string()))?;

        let mut edges = Vec::new();
        let iter = self.db.iterator_cf(&cf_edges, IteratorMode::Start);

        for item in iter {
            let (key, value) = item.map_err(|e| GraphError::RocksDBError(e.to_string()))?;
            let _edge_id = String::from_utf8(key.to_vec())
                .map_err(|e| GraphError::SerializationError(e.to_string()))?;
            let edge: Edge = bincode::deserialize(&value)
                .map_err(|e| GraphError::SerializationError(e.to_string()))?;
            edges.push(edge);
        }

        Ok(edges)
    }

    /// Get database statistics
    #[instrument(skip(self))]
    pub async fn get_statistics(&self) -> Result<GraphStatistics, GraphError> {
        let cf_nodes = self.db.cf_handle("nodes")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("nodes".to_string()))?;
        let cf_edges = self.db.cf_handle("edges")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("edges".to_string()))?;

        // Count nodes
        let mut node_count = 0u64;
        let iter = self.db.iterator_cf(&cf_nodes, IteratorMode::Start);
        for _ in iter {
            node_count += 1;
        }

        // Count edges
        let mut edge_count = 0u64;
        let iter = self.db.iterator_cf(&cf_edges, IteratorMode::Start);
        for _ in iter {
            edge_count += 1;
        }

        Ok(GraphStatistics {
            node_count,
            edge_count,
            cache_size: self.node_cache.len() + self.edge_cache.len(),
        })
    }

    // Internal helper methods
    async fn load_schema(&self) -> Result<(), GraphError> {
        let cf_schema = self.db.cf_handle("schema")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("schema".to_string()))?;

        if let Some(schema_data) = self.db.get_cf(&cf_schema, "schema")
            .map_err(|e| GraphError::RocksDBError(e.to_string()))? {

            let schema: Schema = bincode::deserialize(&schema_data)
                .map_err(|e| GraphError::SerializationError(e.to_string()))?;

            *self.schema.write().await = schema;
        }

        Ok(())
    }

    async fn update_schema_for_node(&self, node: &Node) -> Result<(), GraphError> {
        let mut schema = self.schema.write().await;

        for label in &node.labels {
            let label_schema = schema.node_labels.entry(label.clone()).or_insert_with(|| LabelSchema {
                name: label.clone(),
                properties: HashMap::new(),
            });

            for (prop_name, prop_value) in &node.properties {
                let prop_type = self.infer_property_type(prop_value);
                label_schema.properties.insert(prop_name.clone(), PropertySchema {
                    name: prop_name.clone(),
                    data_type: prop_type,
                });
            }
        }

        // Persist schema
        self.persist_schema(&schema).await?;
        Ok(())
    }

    async fn update_schema_for_edge(&self, edge: &Edge) -> Result<(), GraphError> {
        let mut schema = self.schema.write().await;

        let label_schema = schema.edge_labels.entry(edge.label.clone()).or_insert_with(|| LabelSchema {
            name: edge.label.clone(),
            properties: HashMap::new(),
        });

        for (prop_name, prop_value) in &edge.properties {
            let prop_type = self.infer_property_type(prop_value);
            label_schema.properties.insert(prop_name.clone(), PropertySchema {
                name: prop_name.clone(),
                data_type: prop_type,
            });
        }

        // Persist schema
        self.persist_schema(&schema).await?;
        Ok(())
    }

    fn infer_property_type(&self, value: &PropertyValue) -> PropertyType {
        match value {
            PropertyValue::String(_) => PropertyType::String,
            PropertyValue::Integer(_) => PropertyType::Integer,
            PropertyValue::Float(_) => PropertyType::Float,
            PropertyValue::Boolean(_) => PropertyType::Boolean,
            PropertyValue::Date(_) => PropertyType::Date,
            PropertyValue::List(_) => PropertyType::List,
            PropertyValue::Map(_) => PropertyType::Map,
        }
    }

    async fn persist_schema(&self, schema: &Schema) -> Result<(), GraphError> {
        let cf_schema = self.db.cf_handle("schema")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("schema".to_string()))?;

        let schema_data = bincode::serialize(schema)
            .map_err(|e| GraphError::SerializationError(e.to_string()))?;

        self.db.put_cf(&cf_schema, "schema", schema_data)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        Ok(())
    }

    async fn store_edge_mappings(&self, edge: &Edge) -> Result<(), GraphError> {
        let cf_edges = self.db.cf_handle("edges")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("edges".to_string()))?;

        // Store outgoing mapping
        let outgoing_key = format!("outgoing:{}:{}", edge.from_node, edge.label);
        self.db.put_cf(&cf_edges, &outgoing_key, edge.id.as_bytes())
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        // Store incoming mapping
        let incoming_key = format!("incoming:{}:{}", edge.to_node, edge.label);
        self.db.put_cf(&cf_edges, &incoming_key, edge.id.as_bytes())
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        Ok(())
    }

    async fn create_indexes_for_node(&self, node: &Node) -> Result<(), GraphError> {
        // Create property indexes
        for (prop_name, prop_value) in &node.properties {
            let index_key = format!("prop:node:{}:{}:{}", prop_name, self.property_value_to_string(prop_value), node.id);
            self.store_index(&index_key, &node.id).await?;
        }

        // Create label indexes
        for label in &node.labels {
            let index_key = format!("label:node:{}:{}", label, node.id);
            self.store_index(&index_key, &node.id).await?;
        }

        Ok(())
    }

    async fn create_indexes_for_edge(&self, edge: &Edge) -> Result<(), GraphError> {
        // Create property indexes
        for (prop_name, prop_value) in &edge.properties {
            let index_key = format!("prop:edge:{}:{}:{}", prop_name, self.property_value_to_string(prop_value), edge.id);
            self.store_index(&index_key, &edge.id).await?;
        }

        // Create label index
        let index_key = format!("label:edge:{}:{}", edge.label, edge.id);
        self.store_index(&index_key, &edge.id).await?;

        Ok(())
    }

    async fn store_index(&self, index_key: &str, entity_id: &str) -> Result<(), GraphError> {
        let cf_indexes = self.db.cf_handle("indexes")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("indexes".to_string()))?;

        self.db.put_cf(&cf_indexes, index_key, entity_id)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        Ok(())
    }

    fn property_value_to_string(&self, value: &PropertyValue) -> String {
        match value {
            PropertyValue::String(s) => s.clone(),
            PropertyValue::Integer(i) => i.to_string(),
            PropertyValue::Float(f) => f.to_string(),
            PropertyValue::Boolean(b) => b.to_string(),
            PropertyValue::Date(dt) => dt.to_rfc3339(),
            PropertyValue::List(_) => "[LIST]".to_string(),
            PropertyValue::Map(_) => "[MAP]".to_string(),
        }
    }

    async fn query_nodes_by_pattern(&self, pattern: &NodePattern) -> Result<Vec<Node>, GraphError> {
        // This is a simplified implementation
        // In a real implementation, this would use indexes efficiently
        let cf_nodes = self.db.cf_handle("nodes")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("nodes".to_string()))?;

        let mut nodes = Vec::new();
        let iter = self.db.iterator_cf(&cf_nodes, IteratorMode::Start);

        for item in iter {
            let (_, value) = item.map_err(|e| GraphError::RocksDBError(e.to_string()))?;
            let node: Node = bincode::deserialize(&value)
                .map_err(|e| GraphError::SerializationError(e.to_string()))?;

            // Check if node matches pattern
            if self.node_matches_pattern(&node, pattern) {
                nodes.push(node);
            }
        }

        Ok(nodes)
    }

    fn node_matches_pattern(&self, node: &Node, pattern: &NodePattern) -> bool {
        // Check labels
        if !pattern.labels.is_empty() {
            let node_labels: HashSet<String> = node.labels.iter().cloned().collect();
            let pattern_labels: HashSet<String> = pattern.labels.iter().cloned().collect();
            if !pattern_labels.is_subset(&node_labels) {
                return false;
            }
        }

        // Check properties
        for (prop_name, condition) in &pattern.properties {
            if let Some(node_value) = node.properties.get(prop_name) {
                if !self.property_matches_condition(node_value, condition) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    fn property_matches_condition(&self, value: &PropertyValue, condition: &PropertyCondition) -> bool {
        match condition.operator {
            ComparisonOperator::Equal => value == &condition.value,
            ComparisonOperator::NotEqual => value != &condition.value,
            // Other operators would be implemented here
            _ => false,
        }
    }

    async fn delete_edges_for_node(&self, node_id: &str) -> Result<(), GraphError> {
        // Delete outgoing edges
        let outgoing_edges = self.get_edges_from_node(node_id, None).await?;
        for edge in outgoing_edges {
            self.delete_edge(&edge.id).await?;
        }

        // Delete incoming edges
        let incoming_edges = self.get_edges_to_node(node_id, None).await?;
        for edge in incoming_edges {
            self.delete_edge(&edge.id).await?;
        }

        Ok(())
    }

    async fn delete_edge(&self, edge_id: &str) -> Result<(), GraphError> {
        let cf_edges = self.db.cf_handle("edges")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("edges".to_string()))?;

        let edge_key = format!("edge:{}", edge_id);
        self.db.delete_cf(&cf_edges, &edge_key)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        self.edge_cache.remove(edge_id);
        Ok(())
    }

    async fn update_indexes_for_node(&self, node: &Node) -> Result<(), GraphError> {
        // Delete old indexes and create new ones
        self.delete_indexes_for_node(&node.id).await?;
        self.create_indexes_for_node(node).await?;
        Ok(())
    }

    async fn delete_indexes_for_node(&self, _node_id: &str) -> Result<(), GraphError> {
        // This is a simplified implementation
        // In a real implementation, you'd track and delete specific indexes
        Ok(())
    }
}

/// Graph transaction implementation
impl<'a> GraphTransaction<'a> {
    /// Create a node in the transaction
    pub async fn create_node(
        &mut self,
        id: Option<String>,
        labels: Vec<String>,
        properties: BTreeMap<String, PropertyValue>,
    ) -> Result<String, GraphError> {
        let node_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now();

        let node = Node {
            id: node_id.clone(),
            labels,
            properties,
            created_at: now,
            updated_at: now,
        };

        let node_data = bincode::serialize(&node)
            .map_err(|e| GraphError::SerializationError(e.to_string()))?;

        let cf_nodes = self.db.db.cf_handle("nodes")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("nodes".to_string()))?;

        let node_key = format!("node:{}", node_id);
        self.batch.put_cf(&cf_nodes, &node_key, node_data);

        self.modified_nodes.insert(node_id.clone());

        Ok(node_id)
    }

    /// Create an edge in the transaction
    pub async fn create_edge(
        &mut self,
        id: Option<String>,
        from_node: &str,
        to_node: &str,
        label: String,
        properties: BTreeMap<String, PropertyValue>,
    ) -> Result<String, GraphError> {
        let edge_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now();

        let edge = Edge {
            id: edge_id.clone(),
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            label: label.clone(),
            properties,
            created_at: now,
            updated_at: now,
        };

        let edge_data = bincode::serialize(&edge)
            .map_err(|e| GraphError::SerializationError(e.to_string()))?;

        let cf_edges = self.db.db.cf_handle("edges")
            .ok_or_else(|| GraphError::ColumnFamilyNotFound("edges".to_string()))?;

        let edge_key = format!("edge:{}", edge_id);
        self.batch.put_cf(&cf_edges, &edge_key, edge_data);

        // Store reverse mappings for efficient traversal
        let outgoing_key = format!("outgoing:{}:{}", from_node, label);
        let incoming_key = format!("incoming:{}:{}", to_node, label);
        self.batch.put_cf(&cf_edges, &outgoing_key, edge_id.as_bytes());
        self.batch.put_cf(&cf_edges, &incoming_key, edge_id.as_bytes());

        self.modified_edges.insert(edge_id.clone());

        Ok(edge_id)
    }

    /// Commit the transaction
    pub async fn commit(self) -> Result<(), GraphError> {
        self.db.db.write(self.batch)
            .map_err(|e| GraphError::RocksDBError(e.to_string()))?;

        // Update caches
        for node_id in self.modified_nodes {
            if let Some(node) = self.db.get_node(&node_id).await? {
                self.db.node_cache.insert(node_id, node);
            }
        }

        for edge_id in self.modified_edges {
            if let Some(edge) = self.db.get_edge(&edge_id).await? {
                self.db.edge_cache.insert(edge_id, edge);
            }
        }

        Ok(())
    }

    /// Rollback the transaction
    pub fn rollback(self) -> Result<(), GraphError> {
        // Transaction is automatically rolled back when dropped
        Ok(())
    }
}

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub node_count: u64,
    pub edge_count: u64,
    pub cache_size: usize,
}

/// Graph error types
#[derive(thiserror::Error, Debug)]
pub enum GraphError {
    #[error("RocksDB error: {0}")]
    RocksDBError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Column family not found: {0}")]
    ColumnFamilyNotFound(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Edge not found: {0}")]
    EdgeNotFound(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            node_labels: HashMap::new(),
            edge_labels: HashMap::new(),
            indexes: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_graphdb_creation() {
        let temp_dir = tempdir().unwrap();
        let graphdb = GraphDB::new(temp_dir.path().to_str().unwrap()).await;
        assert!(graphdb.is_ok(), "GraphDB should be created successfully");
    }

    #[tokio::test]
    async fn test_node_operations() {
        let temp_dir = tempdir().unwrap();
        let graphdb = GraphDB::new(temp_dir.path().to_str().unwrap()).await.unwrap();

        // Create node
        let mut properties = BTreeMap::new();
        properties.insert("name".to_string(), PropertyValue::String("Alice".to_string()));
        properties.insert("age".to_string(), PropertyValue::Integer(30));

        let node_id = graphdb.create_node(
            None,
            vec!["Person".to_string()],
            properties,
        ).await.unwrap();

        // Get node
        let node = graphdb.get_node(&node_id).await.unwrap().unwrap();
        assert_eq!(node.labels, vec!["Person"]);
        assert_eq!(node.properties["name"], PropertyValue::String("Alice".to_string()));

        // Update node
        let mut update_props = BTreeMap::new();
        update_props.insert("city".to_string(), PropertyValue::String("Tokyo".to_string()));
        graphdb.update_node(&node_id, update_props).await.unwrap();

        // Verify update
        let updated_node = graphdb.get_node(&node_id).await.unwrap().unwrap();
        assert_eq!(updated_node.properties["city"], PropertyValue::String("Tokyo".to_string()));
    }

    #[tokio::test]
    async fn test_edge_operations() {
        let temp_dir = tempdir().unwrap();
        let graphdb = GraphDB::new(temp_dir.path().to_str().unwrap()).await.unwrap();

        // Create nodes
        let node1_id = graphdb.create_node(
            None,
            vec!["Person".to_string()],
            BTreeMap::new(),
        ).await.unwrap();

        let node2_id = graphdb.create_node(
            None,
            vec!["Person".to_string()],
            BTreeMap::new(),
        ).await.unwrap();

        // Create edge
        let mut edge_props = BTreeMap::new();
        edge_props.insert("since".to_string(), PropertyValue::Integer(2020));

        let edge_id = graphdb.create_edge(
            None,
            &node1_id,
            &node2_id,
            "KNOWS".to_string(),
            edge_props,
        ).await.unwrap();

        // Get edge
        let edge = graphdb.get_edge(&edge_id).await.unwrap().unwrap();
        assert_eq!(edge.from_node, node1_id);
        assert_eq!(edge.to_node, node2_id);
        assert_eq!(edge.label, "KNOWS");

        // Get edges from node
        let outgoing = graphdb.get_edges_from_node(&node1_id, Some("KNOWS")).await.unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].id, edge_id);
    }

    #[tokio::test]
    async fn test_transaction() {
        let temp_dir = tempdir().unwrap();
        let graphdb = GraphDB::new(temp_dir.path().to_str().unwrap()).await.unwrap();

        // Start transaction
        let mut tx = graphdb.begin_transaction().await;

        // Create nodes in transaction
        let node1_id = tx.create_node(
            None,
            vec!["Person".to_string()],
            BTreeMap::new(),
        ).await.unwrap();

        let node2_id = tx.create_node(
            None,
            vec!["Person".to_string()],
            BTreeMap::new(),
        ).await.unwrap();

        // Commit transaction
        tx.commit().await.unwrap();

        // Verify nodes exist
        assert!(graphdb.get_node(&node1_id).await.unwrap().is_some());
        assert!(graphdb.get_node(&node2_id).await.unwrap().is_some());
    }
}
