//! Defines the standard schema for representing UI state as a graph.
//!
//! This includes standard vertex types for UI components, edge labels for
//! relationships (e.g., parent-child), and conventional property keys for state.

/// Standard Vertex Types for UI Components.
pub enum UiVertexType {
    /// The root of the UI component tree.
    UIRoot,
    /// A generic UI component.
    Component,
    /// A modal dialog.
    Modal,
    /// A list of items.
    List,
    /// An input field.
    Input,
    /// A button.
    Button,
    /// A text element.
    Text,
}

impl UiVertexType {
    pub fn as_str(&self) -> &'static str {
        match self {
            UiVertexType::UIRoot => "UIRoot",
            UiVertexType::Component => "Component",
            UiVertexType::Modal => "Modal",
            UiVertexType::List => "List",
            UiVertexType::Input => "Input",
            UiVertexType::Button => "Button",
            UiVertexType::Text => "Text",
        }
    }
}

/// Standard Edge Labels for UI relationships.
pub enum UiEdgeLabel {
    /// Represents a parent-child relationship in the component tree.
    HasChild,
    /// Links a component to its state object.
    HasState,
    /// Links a form to its input fields.
    HasInput,
}

impl UiEdgeLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            UiEdgeLabel::HasChild => "HAS_CHILD",
            UiEdgeLabel::HasState => "HAS_STATE",
            UiEdgeLabel::HasInput => "HAS_INPUT",
        }
    }
}

/// Standard Property Keys for component state.
pub struct UiPropKey;

impl UiPropKey {
    /// The unique identifier for a component within the UI graph.
    pub const ID: &'static str = "id";
    /// The visibility state of a component (boolean).
    pub const IS_VISIBLE: &'static str = "isVisible";
    /// The disabled state of a component (boolean).
    pub const IS_DISABLED: &'static str = "isDisabled";
    /// The title or label of a component (string).
    pub const TITLE: &'static str = "title";
    /// The value of an input component (string, number, etc.).
    pub const VALUE: &'static str = "value";
    /// The items in a list component (array).
    pub const ITEMS: &'static str = "items";
    /// The error message associated with a component (string).
    pub const ERROR: &'static str = "error";
    /// The loading state of a component (boolean).
    pub const IS_LOADING: &'static str = "isLoading";
    /// A map of all other component properties.
    pub const PROPS: &'static str = "props";
}
