//! Renders `kotoba-routing` schema components into HTML strings.

use anyhow::Result;
use kotoba_routing::schema::{Component, ComponentOrString, LayoutModule, PageModule};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default)]
pub struct ComponentRenderer;

impl ComponentRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders a full page, combining layouts and the page component.
    pub fn render_page(
        &self,
        layouts: &[&LayoutModule],
        page: &PageModule,
        props: Value,
    ) -> Result<String> {
        // Start with the innermost page component.
        let mut final_html = self.render_component(&page.component, &props)?;

        // Wrap the result in each layout, from the inside out.
        for layout in layouts.iter().rev() {
            // A special prop `children` is created to contain the inner content.
            let mut layout_props = props.clone();
            if let Value::Object(map) = &mut layout_props {
                map.insert("children".to_string(), Value::String(final_html));
            }
            final_html = self.render_component(&layout.component, &layout_props)?;
        }

        Ok(final_html)
    }

    /// Recursively renders a single component into an HTML string.
    fn render_component(&self, component: &Component, props: &Value) -> Result<String> {
        let tag = &component.component_type;

        // --- Handle children first ---
        let children_html = component
            .children
            .iter()
            .map(|child| match child {
                ComponentOrString::Component(c) => self.render_component(c, props).unwrap_or_default(),
                ComponentOrString::String(s) => self.interpolate(s, props),
            })
            .collect::<String>();

        // --- Handle props ---
        // This is a simplified prop renderer. It doesn't handle complex objects or events.
        let props_str = component
            .props
            .iter()
            .map(|(key, value)| {
                // For simplicity, we assume props are simple strings for now.
                if let Value::String(s) = value {
                    format!(r#" {}="{}""#, key, self.interpolate(s, props))
                } else {
                    "".to_string()
                }
            })
            .collect::<String>();

        // --- Assemble the final tag ---
        // Handle special "children" component type for layout rendering
        if tag == "children" {
            return Ok(props
                .get("children")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string());
        }

        Ok(format!("<{}{}>{}</{}>", tag, props_str, children_html, tag))
    }

    /// A very basic interpolator that replaces `{{ props.user.name }}` style variables.
    fn interpolate(&self, template: &str, props: &Value) -> String {
        // This is a placeholder. A real implementation would use a proper template engine
        // or a more robust regex.
        if template.starts_with("{{") && template.ends_with("}}") {
            let key = template
                .trim_matches(|c| c == '{' || c == '}' || c == ' ')
                .split('.')
                .collect::<Vec<_>>();
            
            // Simplified lookup (e.g., props.user.name)
            if key.get(0) == Some(&"props") {
                return props
                    .get(key[1])
                    .and_then(|v| v.get(key[2]))
                    .and_then(Value::as_str)
                    .unwrap_or("").to_string();
            }
        }
        template.to_string()
    }
}
