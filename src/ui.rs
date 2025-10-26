//! UI Transpiler for Kotoba
//!
//! Converts UI-IR graphs to HTML + Tailwind CSS + HTMX

use crate::{engidb::EngiDB, Error, Result};
use kotoba_types::{Node, UiProperties};
use std::path::Path;

/// UI Transpiler
pub struct UiTranspiler {
    engidb: EngiDB,
}

impl UiTranspiler {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let engidb = EngiDB::open(db_path)?;
        Ok(UiTranspiler { engidb })
    }

    /// Transpile UI-IR to HTML + Tailwind + HTMX
    pub fn transpile_to_html(&self, view_id: &str) -> Result<String> {
        let ui_nodes = self.collect_ui_nodes()?;
        let root_node = ui_nodes.iter()
            .find(|n| n.id == view_id)
            .ok_or_else(|| Error::Validation(format!("View '{}' not found", view_id)))?;

        let html = self.node_to_html(root_node, &ui_nodes)?;
        let full_html = self.wrap_with_template(&html);

        Ok(full_html)
    }

    fn collect_ui_nodes(&self) -> Result<Vec<Node>> {
        // For now, return mock UI nodes since full query isn't implemented
        // In the future, this will query EngiDB for UiNodeType nodes
        Ok(self.create_mock_todo_ui())
    }

    fn node_to_html(&self, node: &Node, all_nodes: &[Node]) -> Result<String> {
        // Parse UI properties from node
        let ui_props: UiProperties = serde_json::from_value(
            serde_json::to_value(&node.properties)
                .map_err(|e| Error::Validation(e.to_string()))?
        ).map_err(|e| Error::Validation(e.to_string()))?;

        let mut html = String::new();

        // HTML tag
        let tag = ui_props.html_tag.unwrap_or_else(|| "div".to_string());
        html.push_str(&format!("<{}", tag));

        // Attributes
        for (key, value) in &ui_props.attributes {
            html.push_str(&format!(" {}=\"{}\"", key, value));
        }

        // HTMX attributes
        for (key, value) in &ui_props.htmx_attrs {
            html.push_str(&format!(" {}=\"{}\"", key, value));
        }

        // Tailwind classes
        if !ui_props.tailwind_classes.is_empty() {
            let classes = ui_props.tailwind_classes.join(" ");
            html.push_str(&format!(" class=\"{}\"", classes));
        }

        html.push('>');

        // Content
        if let Some(content) = &ui_props.content {
            html.push_str(content);
        }

        // Children
        for child_id in &ui_props.children {
            if let Some(child_node) = all_nodes.iter().find(|n| n.id == *child_id) {
                let child_html = self.node_to_html(child_node, all_nodes)?;
                html.push_str(&child_html);
            }
        }

        html.push_str(&format!("</{}>", tag));

        Ok(html)
    }

    fn wrap_with_template(&self, body_html: &str) -> String {
        format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kotoba Todo App</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <style>
        .completed {{ text-decoration: line-through; opacity: 0.6; }}
    </style>
</head>
<body class="bg-gray-100 min-h-screen py-8">
    <div class="max-w-2xl mx-auto bg-white rounded-lg shadow-md p-6">
        <h1 class="text-3xl font-bold text-gray-800 mb-6">Kotoba Todo App</h1>
        {}
    </div>
</body>
</html>"#, body_html)
    }

    fn create_mock_todo_ui(&self) -> Vec<Node> {
        vec![
            Node {
                id: "todo_view".to_string(),
                kind: "View".to_string(),
                properties: {
                    let mut props = indexmap::IndexMap::new();
                    props.insert("node_type".to_string(), serde_json::json!("View"));
                    props.insert("html_tag".to_string(), serde_json::json!("div"));
                    props.insert("tailwind_classes".to_string(), serde_json::json!(["space-y-4"]));
                    props.insert("children".to_string(), serde_json::json!(["todo_form", "todo_list"]));
                    props
                },
            },
            Node {
                id: "todo_form".to_string(),
                kind: "Component".to_string(),
                properties: {
                    let mut props = indexmap::IndexMap::new();
                    props.insert("node_type".to_string(), serde_json::json!("Component"));
                    props.insert("html_tag".to_string(), serde_json::json!("form"));
                    props.insert("tailwind_classes".to_string(), serde_json::json!(["flex", "gap-2", "mb-6"]));
                    props.insert("htmx_attrs".to_string(), serde_json::json!({"hx-post": "/api/todo/add", "hx-target": "#todo_list", "hx-swap": "innerHTML"}));
                    props.insert("children".to_string(), serde_json::json!(["todo_input", "todo_submit"]));
                    props
                },
            },
            Node {
                id: "todo_input".to_string(),
                kind: "Component".to_string(),
                properties: {
                    let mut props = indexmap::IndexMap::new();
                    props.insert("node_type".to_string(), serde_json::json!("Component"));
                    props.insert("html_tag".to_string(), serde_json::json!("input"));
                    props.insert("tailwind_classes".to_string(), serde_json::json!(["flex-1", "px-4", "py-2", "border", "border-gray-300", "rounded-lg", "focus:outline-none", "focus:ring-2", "focus:ring-blue-500"]));
                    props.insert("attributes".to_string(), serde_json::json!({"type": "text", "name": "title", "placeholder": "Add a new todo...", "required": "true"}));
                    props
                },
            },
            Node {
                id: "todo_submit".to_string(),
                kind: "Component".to_string(),
                properties: {
                    let mut props = indexmap::IndexMap::new();
                    props.insert("node_type".to_string(), serde_json::json!("Component"));
                    props.insert("html_tag".to_string(), serde_json::json!("button"));
                    props.insert("tailwind_classes".to_string(), serde_json::json!(["px-6", "py-2", "bg-blue-500", "text-white", "rounded-lg", "hover:bg-blue-600", "focus:outline-none", "focus:ring-2", "focus:ring-blue-500"]));
                    props.insert("attributes".to_string(), serde_json::json!({"type": "submit"}));
                    props.insert("content".to_string(), serde_json::json!("Add Todo"));
                    props
                },
            },
            Node {
                id: "todo_list".to_string(),
                kind: "Component".to_string(),
                properties: {
                    let mut props = indexmap::IndexMap::new();
                    props.insert("node_type".to_string(), serde_json::json!("Component"));
                    props.insert("html_tag".to_string(), serde_json::json!("div"));
                    props.insert("attributes".to_string(), serde_json::json!({"id": "todo_list"}));
                    props.insert("htmx_attrs".to_string(), serde_json::json!({"hx-get": "/api/todo/list", "hx-trigger": "load, todoAdded from:body"}));
                    props.insert("content".to_string(), serde_json::json!("<div class=\"text-center text-gray-500\">Loading todos...</div>"));
                    props
                },
            },
        ]
    }
}
