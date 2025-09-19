//! `kotoba-ocel`
//!
//! OCEL v2 (Object-Centric Event Log) implementation for KotobaDB.
//! Provides object-centric event log format for process mining and analysis.

use std::collections::{HashMap, HashSet, BTreeMap};
use serde::{Deserialize, Serialize};
use indexmap::IndexMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Core OCEL v2 event log structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelLog {
    /// Global attributes of the log
    pub global_log: ValueMap,
    /// Global event attributes
    pub global_event: ValueMap,
    /// Global object attributes
    pub global_object: ValueMap,
    /// Events in the log
    pub events: IndexMap<String, OcelEvent>,
    /// Objects in the log
    pub objects: IndexMap<String, OcelObject>,
}

/// OCEL v2 Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelEvent {
    /// Event ID
    pub id: String,
    /// Activity name
    pub activity: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event attributes (value map)
    pub vmap: ValueMap,
    /// Related objects (object map)
    pub omap: Vec<String>,
}

/// OCEL v2 Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelObject {
    /// Object ID
    pub id: String,
    /// Object type
    pub object_type: String,
    /// Object attributes (value map)
    pub vmap: ValueMap,
}

/// Value map for attributes (flexible value storage)
pub type ValueMap = IndexMap<String, OcelValue>;

/// OCEL value types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum OcelValue {
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
    List(Vec<OcelValue>),
    /// Map of values
    Map(ValueMap),
}

/// OCEL v2 Object Type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelObjectType {
    /// Object type name
    pub name: String,
    /// Attribute definitions
    pub attributes: IndexMap<String, AttributeDefinition>,
}

/// OCEL v2 Event Type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelEventType {
    /// Event type (activity) name
    pub name: String,
    /// Attribute definitions
    pub attributes: IndexMap<String, AttributeDefinition>,
}

/// Attribute definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    /// Attribute name
    pub name: String,
    /// Attribute type
    pub attr_type: AttributeType,
}

/// Attribute types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeType {
    /// String attribute
    String,
    /// Integer attribute
    Integer,
    /// Float attribute
    Float,
    /// Boolean attribute
    Boolean,
    /// Date/time attribute
    Date,
    /// List attribute
    List(Box<AttributeType>),
}

/// Event-Object relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcelRelationship {
    /// Event ID
    pub event_id: String,
    /// Object ID
    pub object_id: String,
    /// Relationship qualifier (optional)
    pub qualifier: Option<String>,
}

impl OcelLog {
    /// Create a new empty OCEL log
    pub fn new() -> Self {
        Self {
            global_log: ValueMap::new(),
            global_event: ValueMap::new(),
            global_object: ValueMap::new(),
            events: IndexMap::new(),
            objects: IndexMap::new(),
        }
    }

    /// Add an event to the log
    pub fn add_event(&mut self, event: OcelEvent) -> Result<(), OcelError> {
        if self.events.contains_key(&event.id) {
            return Err(OcelError::DuplicateEventId(event.id));
        }
        self.events.insert(event.id.clone(), event);
        Ok(())
    }

    /// Add an object to the log
    pub fn add_object(&mut self, object: OcelObject) -> Result<(), OcelError> {
        if self.objects.contains_key(&object.id) {
            return Err(OcelError::DuplicateObjectId(object.id));
        }
        self.objects.insert(object.id.clone(), object);
        Ok(())
    }

    /// Get events for a specific object
    pub fn get_events_for_object(&self, object_id: &str) -> Vec<&OcelEvent> {
        self.events.values()
            .filter(|event| event.omap.contains(&object_id.to_string()))
            .collect()
    }

    /// Get objects for a specific event
    pub fn get_objects_for_event(&self, event_id: &str) -> Vec<&OcelObject> {
        if let Some(event) = self.events.get(event_id) {
            event.omap.iter()
                .filter_map(|obj_id| self.objects.get(obj_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get events by activity
    pub fn get_events_by_activity(&self, activity: &str) -> Vec<&OcelEvent> {
        self.events.values()
            .filter(|event| event.activity == activity)
            .collect()
    }

    /// Get objects by type
    pub fn get_objects_by_type(&self, object_type: &str) -> Vec<&OcelObject> {
        self.objects.values()
            .filter(|obj| obj.object_type == object_type)
            .collect()
    }

    /// Get unique activities
    pub fn get_activities(&self) -> HashSet<String> {
        self.events.values()
            .map(|event| event.activity.clone())
            .collect()
    }

    /// Get unique object types
    pub fn get_object_types(&self) -> HashSet<String> {
        self.objects.values()
            .map(|obj| obj.object_type.clone())
            .collect()
    }

    /// Validate the log structure
    pub fn validate(&self) -> Result<(), OcelError> {
        // Check that all objects referenced in events exist
        for event in self.events.values() {
            for obj_id in &event.omap {
                if !self.objects.contains_key(obj_id) {
                    return Err(OcelError::MissingObject(obj_id.clone()));
                }
            }
        }

        // Check for duplicate event IDs
        let mut event_ids = HashSet::new();
        for event_id in self.events.keys() {
            if !event_ids.insert(event_id) {
                return Err(OcelError::DuplicateEventId(event_id.clone()));
            }
        }

        // Check for duplicate object IDs
        let mut object_ids = HashSet::new();
        for object_id in self.objects.keys() {
            if !object_ids.insert(object_id) {
                return Err(OcelError::DuplicateObjectId(object_id.clone()));
            }
        }

        Ok(())
    }
}

impl OcelEvent {
    /// Create a new event
    pub fn new(id: String, activity: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            id,
            activity,
            timestamp,
            vmap: ValueMap::new(),
            omap: Vec::new(),
        }
    }

    /// Add an attribute to the event
    pub fn with_attribute(mut self, key: String, value: OcelValue) -> Self {
        self.vmap.insert(key, value);
        self
    }

    /// Add a related object to the event
    pub fn with_object(mut self, object_id: String) -> Self {
        self.omap.push(object_id);
        self
    }
}

impl OcelObject {
    /// Create a new object
    pub fn new(id: String, object_type: String) -> Self {
        Self {
            id,
            object_type,
            vmap: ValueMap::new(),
        }
    }

    /// Add an attribute to the object
    pub fn with_attribute(mut self, key: String, value: OcelValue) -> Self {
        self.vmap.insert(key, value);
        self
    }
}

impl Default for OcelValue {
    fn default() -> Self {
        OcelValue::String(String::new())
    }
}

/// OCEL error types
#[derive(thiserror::Error, Debug)]
pub enum OcelError {
    #[error("Missing object: {0}")]
    MissingObject(String),

    #[error("Duplicate event ID: {0}")]
    DuplicateEventId(String),

    #[error("Duplicate object ID: {0}")]
    DuplicateObjectId(String),

    #[error("Invalid attribute type")]
    InvalidAttributeType,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// OCEL v2 log builder
pub struct OcelLogBuilder {
    log: OcelLog,
}

impl OcelLogBuilder {
    /// Create a new log builder
    pub fn new() -> Self {
        Self {
            log: OcelLog::new(),
        }
    }

    /// Set global log attributes
    pub fn global_log_attribute(mut self, key: String, value: OcelValue) -> Self {
        self.log.global_log.insert(key, value);
        self
    }

    /// Set global event attributes
    pub fn global_event_attribute(mut self, key: String, value: OcelValue) -> Self {
        self.log.global_event.insert(key, value);
        self
    }

    /// Set global object attributes
    pub fn global_object_attribute(mut self, key: String, value: OcelValue) -> Self {
        self.log.global_object.insert(key, value);
        self
    }

    /// Add an event
    pub fn event(mut self, event: OcelEvent) -> Self {
        self.log.add_event(event).unwrap(); // Builder assumes valid input
        self
    }

    /// Add an object
    pub fn object(mut self, object: OcelObject) -> Self {
        self.log.add_object(object).unwrap(); // Builder assumes valid input
        self
    }

    /// Build the OCEL log
    pub fn build(self) -> Result<OcelLog, OcelError> {
        self.log.validate()?;
        Ok(self.log)
    }
}

/// Utility functions for OCEL v2
pub mod utils {
    use super::*;

    /// Convert serde_json::Value to OcelValue
    pub fn json_to_ocel_value(value: &serde_json::Value) -> OcelValue {
        match value {
            serde_json::Value::String(s) => OcelValue::String(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    OcelValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    OcelValue::Float(f)
                } else {
                    OcelValue::String(n.to_string())
                }
            }
            serde_json::Value::Bool(b) => OcelValue::Boolean(*b),
            serde_json::Value::Array(arr) => {
                let ocel_arr = arr.iter().map(json_to_ocel_value).collect();
                OcelValue::List(ocel_arr)
            }
            serde_json::Value::Object(obj) => {
                let mut ocel_map = ValueMap::new();
                for (k, v) in obj {
                    ocel_map.insert(k.clone(), json_to_ocel_value(v));
                }
                OcelValue::Map(ocel_map)
            }
            serde_json::Value::Null => OcelValue::String("null".to_string()),
        }
    }

    /// Convert OcelValue to serde_json::Value
    pub fn ocel_to_json_value(value: &OcelValue) -> serde_json::Value {
        match value {
            OcelValue::String(s) => serde_json::Value::String(s.clone()),
            OcelValue::Integer(i) => serde_json::json!(i),
            OcelValue::Float(f) => serde_json::json!(f),
            OcelValue::Boolean(b) => serde_json::json!(b),
            OcelValue::Date(dt) => serde_json::json!(dt.to_rfc3339()),
            OcelValue::List(arr) => {
                let json_arr = arr.iter().map(ocel_to_json_value).collect();
                serde_json::Value::Array(json_arr)
            }
            OcelValue::Map(map) => {
                let mut json_obj = serde_json::Map::new();
                for (k, v) in map {
                    json_obj.insert(k.clone(), ocel_to_json_value(v));
                }
                serde_json::Value::Object(json_obj)
            }
        }
    }

    /// Generate a unique ID
    pub fn generate_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Create an event from JSON data
    pub fn event_from_json(json: &serde_json::Value) -> Result<OcelEvent, OcelError> {
        let id = json.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OcelError::ValidationError("Missing event ID".to_string()))?
            .to_string();

        let activity = json.get("activity")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OcelError::ValidationError("Missing activity".to_string()))?
            .to_string();

        let timestamp_str = json.get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OcelError::ValidationError("Missing timestamp".to_string()))?;

        let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
            .map_err(|_| OcelError::ValidationError("Invalid timestamp format".to_string()))?
            .with_timezone(&Utc);

        let mut event = OcelEvent::new(id, activity, timestamp);

        // Add vmap attributes
        if let Some(vmap) = json.get("vmap").and_then(|v| v.as_object()) {
            for (key, value) in vmap {
                event.vmap.insert(key.clone(), json_to_ocel_value(value));
            }
        }

        // Add object mappings
        if let Some(omap) = json.get("omap").and_then(|v| v.as_array()) {
            for obj_id in omap {
                if let Some(obj_id_str) = obj_id.as_str() {
                    event.omap.push(obj_id_str.to_string());
                }
            }
        }

        Ok(event)
    }

    /// Create an object from JSON data
    pub fn object_from_json(json: &serde_json::Value) -> Result<OcelObject, OcelError> {
        let id = json.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OcelError::ValidationError("Missing object ID".to_string()))?
            .to_string();

        let object_type = json.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OcelError::ValidationError("Missing object type".to_string()))?
            .to_string();

        let mut object = OcelObject::new(id, object_type);

        // Add vmap attributes
        if let Some(vmap) = json.get("vmap").and_then(|v| v.as_object()) {
            for (key, value) in vmap {
                object.vmap.insert(key.clone(), json_to_ocel_value(value));
            }
        }

        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_ocel_log_creation() {
        let mut log = OcelLog::new();

        // Add an object
        let object = OcelObject::new("obj1".to_string(), "Order".to_string())
            .with_attribute("amount".to_string(), OcelValue::Float(100.0));
        log.add_object(object).unwrap();

        // Add an event
        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let mut event = OcelEvent::new("evt1".to_string(), "create_order".to_string(), timestamp);
        event.omap.push("obj1".to_string());
        log.add_event(event).unwrap();

        // Validate log
        assert!(log.validate().is_ok());

        // Test queries
        let events_for_obj = log.get_events_for_object("obj1");
        assert_eq!(events_for_obj.len(), 1);
        assert_eq!(events_for_obj[0].activity, "create_order");

        let objects_for_event = log.get_objects_for_event("evt1");
        assert_eq!(objects_for_event.len(), 1);
        assert_eq!(objects_for_event[0].object_type, "Order");
    }

    #[test]
    fn test_ocel_log_builder() {
        let log = OcelLogBuilder::new()
            .global_log_attribute("name".to_string(), OcelValue::String("Test Log".to_string()))
            .event(OcelEvent::new("evt1".to_string(), "test_activity".to_string(), Utc::now()))
            .object(OcelObject::new("obj1".to_string(), "TestObject".to_string()))
            .build();

        assert!(log.is_ok());
        let log = log.unwrap();
        assert_eq!(log.events.len(), 1);
        assert_eq!(log.objects.len(), 1);
    }

    #[test]
    fn test_value_conversion() {
        use utils::*;

        // Test JSON to OCEL conversion
        let json = serde_json::json!({
            "name": "test",
            "count": 42,
            "price": 99.99,
            "active": true,
            "items": ["a", "b", "c"],
            "metadata": {"key": "value"}
        });

        let ocel_value = json_to_ocel_value(&json);
        if let OcelValue::Map(ref map) = ocel_value {
            assert_eq!(map.get("name"), Some(&OcelValue::String("test".to_string())));
            assert_eq!(map.get("count"), Some(&OcelValue::Integer(42)));
            assert_eq!(map.get("price"), Some(&OcelValue::Float(99.99)));
            assert_eq!(map.get("active"), Some(&OcelValue::Boolean(true)));
        } else {
            panic!("Expected Map");
        }

        // Test OCEL to JSON conversion
        let json_value = ocel_to_json_value(&ocel_value);
        assert!(json_value.is_object());
    }

    #[test]
    fn test_validation_errors() {
        let mut log = OcelLog::new();

        // Add event with non-existent object
        let mut event = OcelEvent::new("evt1".to_string(), "test".to_string(), Utc::now());
        event.omap.push("non_existent_obj".to_string());
        log.add_event(event).unwrap();

        // Validation should fail
        assert!(log.validate().is_err());
    }

    #[test]
    fn test_ocel_event_creation() {
        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let event = OcelEvent::new("evt1".to_string(), "create_order".to_string(), timestamp);

        assert_eq!(event.id, "evt1");
        assert_eq!(event.activity, "create_order");
        assert_eq!(event.timestamp, timestamp);
        assert!(event.vmap.is_empty());
        assert!(event.omap.is_empty());
    }

    #[test]
    fn test_ocel_event_with_attributes() {
        let timestamp = Utc::now();
        let event = OcelEvent::new("evt1".to_string(), "create_order".to_string(), timestamp)
            .with_attribute("amount".to_string(), OcelValue::Float(100.0))
            .with_attribute("currency".to_string(), OcelValue::String("USD".to_string()))
            .with_object("obj1".to_string())
            .with_object("obj2".to_string());

        assert_eq!(event.vmap.len(), 2);
        assert_eq!(event.vmap.get("amount"), Some(&OcelValue::Float(100.0)));
        assert_eq!(event.omap.len(), 2);
        assert!(event.omap.contains(&"obj1".to_string()));
        assert!(event.omap.contains(&"obj2".to_string()));
    }

    #[test]
    fn test_ocel_object_creation() {
        let object = OcelObject::new("obj1".to_string(), "Order".to_string());

        assert_eq!(object.id, "obj1");
        assert_eq!(object.object_type, "Order");
        assert!(object.vmap.is_empty());
    }

    #[test]
    fn test_ocel_object_with_attributes() {
        let object = OcelObject::new("obj1".to_string(), "Order".to_string())
            .with_attribute("amount".to_string(), OcelValue::Float(100.0))
            .with_attribute("status".to_string(), OcelValue::String("pending".to_string()));

        assert_eq!(object.vmap.len(), 2);
        assert_eq!(object.vmap.get("amount"), Some(&OcelValue::Float(100.0)));
        assert_eq!(object.vmap.get("status"), Some(&OcelValue::String("pending".to_string())));
    }

    #[test]
    fn test_ocel_value_types() {
        // Test all OCEL value types
        assert_eq!(OcelValue::String("hello".to_string()), OcelValue::String("hello".to_string()));
        assert_eq!(OcelValue::Integer(42), OcelValue::Integer(42));
        assert_eq!(OcelValue::Float(3.14), OcelValue::Float(3.14));
        assert_eq!(OcelValue::Boolean(true), OcelValue::Boolean(true));

        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        assert_eq!(OcelValue::Date(timestamp), OcelValue::Date(timestamp));

        let list = vec![OcelValue::Integer(1), OcelValue::Integer(2)];
        assert_eq!(OcelValue::List(list.clone()), OcelValue::List(list));

        let mut map = ValueMap::new();
        map.insert("key".to_string(), OcelValue::String("value".to_string()));
        assert_eq!(OcelValue::Map(map.clone()), OcelValue::Map(map));
    }

    #[test]
    fn test_ocel_log_queries() {
        let mut log = OcelLog::new();

        // Create objects
        let order = OcelObject::new("order1".to_string(), "Order".to_string())
            .with_attribute("amount".to_string(), OcelValue::Float(100.0));
        log.add_object(order).unwrap();

        let customer = OcelObject::new("customer1".to_string(), "Customer".to_string())
            .with_attribute("name".to_string(), OcelValue::String("Alice".to_string()));
        log.add_object(customer).unwrap();

        // Create events
        let timestamp1 = Utc.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();
        let event1 = OcelEvent::new("evt1".to_string(), "create_order".to_string(), timestamp1)
            .with_object("order1".to_string())
            .with_object("customer1".to_string());
        log.add_event(event1).unwrap();

        let timestamp2 = Utc.with_ymd_and_hms(2023, 1, 1, 11, 0, 0).unwrap();
        let event2 = OcelEvent::new("evt2".to_string(), "process_payment".to_string(), timestamp2)
            .with_object("order1".to_string());
        log.add_event(event2).unwrap();

        let timestamp3 = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let event3 = OcelEvent::new("evt3".to_string(), "create_order".to_string(), timestamp3)
            .with_object("customer1".to_string());
        log.add_event(event3).unwrap();

        // Test queries
        let events_for_order = log.get_events_for_object("order1");
        assert_eq!(events_for_order.len(), 2);

        let events_for_customer = log.get_events_for_object("customer1");
        assert_eq!(events_for_customer.len(), 2);

        let objects_for_evt1 = log.get_objects_for_event("evt1");
        assert_eq!(objects_for_evt1.len(), 2);

        let create_order_events = log.get_events_by_activity("create_order");
        assert_eq!(create_order_events.len(), 2);

        let process_payment_events = log.get_events_by_activity("process_payment");
        assert_eq!(process_payment_events.len(), 1);

        let order_objects = log.get_objects_by_type("Order");
        assert_eq!(order_objects.len(), 1);

        let customer_objects = log.get_objects_by_type("Customer");
        assert_eq!(customer_objects.len(), 1);

        let activities = log.get_activities();
        assert_eq!(activities.len(), 2);
        assert!(activities.contains("create_order"));
        assert!(activities.contains("process_payment"));

        let object_types = log.get_object_types();
        assert_eq!(object_types.len(), 2);
        assert!(object_types.contains("Order"));
        assert!(object_types.contains("Customer"));
    }

    #[test]
    fn test_ocel_log_validation() {
        // Test valid log
        let mut log = OcelLog::new();

        let object = OcelObject::new("obj1".to_string(), "Test".to_string());
        log.add_object(object).unwrap();

        let event = OcelEvent::new("evt1".to_string(), "test".to_string(), Utc::now())
            .with_object("obj1".to_string());
        log.add_event(event).unwrap();

        assert!(log.validate().is_ok());
    }

    #[test]
    fn test_ocel_log_validation_missing_object() {
        let mut log = OcelLog::new();

        let event = OcelEvent::new("evt1".to_string(), "test".to_string(), Utc::now())
            .with_object("non_existent".to_string());
        log.add_event(event);

        let result = log.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            OcelError::MissingObject(_) => {},
            _ => panic!("Expected MissingObject error"),
        }
    }

    #[test]
    fn test_ocel_log_validation_duplicate_ids() {
        let mut log = OcelLog::new();

        let obj1 = OcelObject::new("obj1".to_string(), "Test".to_string());
        let obj2 = OcelObject::new("obj1".to_string(), "Test".to_string()); // Same ID

        // First addition should succeed
        assert!(log.add_object(obj1).is_ok());

        // Second addition with same ID should fail
        let result = log.add_object(obj2);
        assert!(result.is_err(), "Expected duplicate object ID error");
        match result.unwrap_err() {
            OcelError::DuplicateObjectId(_) => {},
            err => panic!("Expected DuplicateObjectId error, got: {:?}", err),
        }
    }

    #[test]
    fn test_ocel_log_builder_comprehensive() {
        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();

        let log = OcelLogBuilder::new()
            .global_log_attribute("version".to_string(), OcelValue::String("1.0".to_string()))
            .global_event_attribute("source".to_string(), OcelValue::String("test".to_string()))
            .global_object_attribute("system".to_string(), OcelValue::String("test".to_string()))
            .object(
                OcelObject::new("obj1".to_string(), "Order".to_string())
                    .with_attribute("amount".to_string(), OcelValue::Float(100.0))
            )
            .event(
                OcelEvent::new("evt1".to_string(), "create_order".to_string(), timestamp)
                    .with_attribute("urgent".to_string(), OcelValue::Boolean(true))
                    .with_object("obj1".to_string())
            )
            .build();

        assert!(log.is_ok());
        let log = log.unwrap();

        assert_eq!(log.global_log.len(), 1);
        assert_eq!(log.global_event.len(), 1);
        assert_eq!(log.global_object.len(), 1);
        assert_eq!(log.events.len(), 1);
        assert_eq!(log.objects.len(), 1);

        let event = log.events.get("evt1").unwrap();
        assert_eq!(event.activity, "create_order");
        assert_eq!(event.omap, vec!["obj1"]);

        let object = log.objects.get("obj1").unwrap();
        assert_eq!(object.object_type, "Order");
    }

    #[test]
    fn test_json_conversion_roundtrip() {
        use utils::*;

        // Create a complex OCEL value
        let mut map = ValueMap::new();
        map.insert("name".to_string(), OcelValue::String("test".to_string()));
        map.insert("count".to_string(), OcelValue::Integer(42));
        map.insert("items".to_string(), OcelValue::List(vec![
            OcelValue::String("a".to_string()),
            OcelValue::String("b".to_string()),
        ]));

        let mut nested_map = ValueMap::new();
        nested_map.insert("nested".to_string(), OcelValue::Boolean(true));
        map.insert("metadata".to_string(), OcelValue::Map(nested_map));

        let ocel_value = OcelValue::Map(map);

        // Convert to JSON and back
        let json_value = ocel_to_json_value(&ocel_value);
        let ocel_value2 = json_to_ocel_value(&json_value);

        // They should be equal
        assert_eq!(ocel_value, ocel_value2);
    }

    #[test]
    fn test_event_from_json() {
        use utils::*;

        let json = serde_json::json!({
            "id": "evt1",
            "activity": "create_order",
            "timestamp": "2023-01-01T12:00:00Z",
            "vmap": {
                "amount": 100.0,
                "urgent": true
            },
            "omap": ["obj1", "obj2"]
        });

        let result = event_from_json(&json);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.id, "evt1");
        assert_eq!(event.activity, "create_order");
        assert_eq!(event.omap, vec!["obj1", "obj2"]);
        assert_eq!(event.vmap.get("amount"), Some(&OcelValue::Float(100.0)));
        assert_eq!(event.vmap.get("urgent"), Some(&OcelValue::Boolean(true)));
    }

    #[test]
    fn test_object_from_json() {
        use utils::*;

        let json = serde_json::json!({
            "id": "obj1",
            "type": "Order",
            "vmap": {
                "amount": 100.0,
                "status": "pending"
            }
        });

        let result = object_from_json(&json);
        assert!(result.is_ok());

        let object = result.unwrap();
        assert_eq!(object.id, "obj1");
        assert_eq!(object.object_type, "Order");
        assert_eq!(object.vmap.get("amount"), Some(&OcelValue::Float(100.0)));
        assert_eq!(object.vmap.get("status"), Some(&OcelValue::String("pending".to_string())));
    }

    #[test]
    fn test_utils_generate_id() {
        use utils::*;

        let id1 = generate_id();
        let id2 = generate_id();

        // IDs should be different
        assert_ne!(id1, id2);

        // IDs should be valid UUIDs
        assert!(uuid::Uuid::parse_str(&id1).is_ok());
        assert!(uuid::Uuid::parse_str(&id2).is_ok());
    }

    #[test]
    fn test_ocel_error_types() {
        let missing_obj = OcelError::MissingObject("obj1".to_string());
        assert!(format!("{}", missing_obj).contains("obj1"));

        let duplicate_event = OcelError::DuplicateEventId("evt1".to_string());
        assert!(format!("{}", duplicate_event).contains("evt1"));

        let duplicate_obj = OcelError::DuplicateObjectId("obj1".to_string());
        assert!(format!("{}", duplicate_obj).contains("obj1"));

        let validation_err = OcelError::ValidationError("test error".to_string());
        assert!(format!("{}", validation_err).contains("test error"));
    }

    #[test]
    fn test_complex_ocel_log() {
        // Create a more complex log representing a business process
        let mut log = OcelLog::new();

        // Add global attributes
        log.global_log.insert("process_name".to_string(), OcelValue::String("Order Fulfillment".to_string()));
        log.global_log.insert("version".to_string(), OcelValue::String("1.0".to_string()));

        // Create objects
        let order = OcelObject::new("order_001".to_string(), "Order".to_string())
            .with_attribute("customer_id".to_string(), OcelValue::String("cust_123".to_string()))
            .with_attribute("total_amount".to_string(), OcelValue::Float(299.99))
            .with_attribute("items".to_string(), OcelValue::List(vec![
                OcelValue::String("item_001".to_string()),
                OcelValue::String("item_002".to_string()),
            ]));
        log.add_object(order).unwrap();

        let customer = OcelObject::new("cust_123".to_string(), "Customer".to_string())
            .with_attribute("name".to_string(), OcelValue::String("Alice Johnson".to_string()))
            .with_attribute("email".to_string(), OcelValue::String("alice@example.com".to_string()));
        log.add_object(customer).unwrap();

        let item1 = OcelObject::new("item_001".to_string(), "Item".to_string())
            .with_attribute("name".to_string(), OcelValue::String("Laptop".to_string()))
            .with_attribute("price".to_string(), OcelValue::Float(999.99));
        log.add_object(item1).unwrap();

        let item2 = OcelObject::new("item_002".to_string(), "Item".to_string())
            .with_attribute("name".to_string(), OcelValue::String("Mouse".to_string()))
            .with_attribute("price".to_string(), OcelValue::Float(29.99));
        log.add_object(item2).unwrap();

        // Create events representing the process
        let timestamp1 = Utc.with_ymd_and_hms(2023, 1, 1, 10, 0, 0).unwrap();
        let create_order = OcelEvent::new("evt_001".to_string(), "create_order".to_string(), timestamp1)
            .with_attribute("channel".to_string(), OcelValue::String("web".to_string()))
            .with_object("order_001".to_string())
            .with_object("cust_123".to_string());
        log.add_event(create_order).unwrap();

        let timestamp2 = Utc.with_ymd_and_hms(2023, 1, 1, 10, 15, 0).unwrap();
        let process_payment = OcelEvent::new("evt_002".to_string(), "process_payment".to_string(), timestamp2)
            .with_attribute("payment_method".to_string(), OcelValue::String("credit_card".to_string()))
            .with_attribute("amount".to_string(), OcelValue::Float(299.99))
            .with_object("order_001".to_string());
        log.add_event(process_payment).unwrap();

        let timestamp3 = Utc.with_ymd_and_hms(2023, 1, 1, 11, 0, 0).unwrap();
        let ship_order = OcelEvent::new("evt_003".to_string(), "ship_order".to_string(), timestamp3)
            .with_attribute("carrier".to_string(), OcelValue::String("UPS".to_string()))
            .with_attribute("tracking_number".to_string(), OcelValue::String("1Z999AA1234567890".to_string()))
            .with_object("order_001".to_string())
            .with_object("item_001".to_string())
            .with_object("item_002".to_string());
        log.add_event(ship_order).unwrap();

        // Validate the log
        assert!(log.validate().is_ok());

        // Test comprehensive queries
        assert_eq!(log.events.len(), 3);
        assert_eq!(log.objects.len(), 4);
        assert_eq!(log.get_activities().len(), 3);
        assert_eq!(log.get_object_types().len(), 3);

        let order_events = log.get_events_for_object("order_001");
        assert_eq!(order_events.len(), 3);

        let customer_events = log.get_events_for_object("cust_123");
        assert_eq!(customer_events.len(), 1);

        let create_events = log.get_events_by_activity("create_order");
        assert_eq!(create_events.len(), 1);

        let order_objects = log.get_objects_by_type("Order");
        assert_eq!(order_objects.len(), 1);

        let item_objects = log.get_objects_by_type("Item");
        assert_eq!(item_objects.len(), 2);
    }
}
