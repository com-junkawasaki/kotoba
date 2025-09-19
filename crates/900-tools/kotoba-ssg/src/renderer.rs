//! Renders `kotoba-routing` schema components into HTML strings.

use anyhow::Result;
use kotoba_routing::schema::{Component, ComponentOrString, LayoutModule, PageModule};
use serde_json::Value;
use std::collections::HashMap;
use kotoba_jsonnet::{evaluate, EvaluationResult, FileResolver};

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

    /// Evaluates a Jsonnet expression string with props as context.
    fn interpolate(&self, template: &str, props: &Value) -> String {
        // We wrap the template in a function and call it with `props`
        // to make the `props` object available in the expression.
        let expr = format!(r#"
            function(props)
                {template}
        "#);

        let props_json = serde_json::to_string(props).unwrap_or_else(|_| "{}".to_string());
        
        // Use a simple resolver as we are not dealing with file imports here.
        let resolver = FileResolver::default();
        
        match evaluate(&expr, &resolver) {
            Ok(EvaluationResult::Str(evaluated_func)) => {
                // Now, evaluate the function call with the actual props
                let final_expr = format!("({})({})", evaluated_func, props_json);
                match evaluate(&final_expr, &resolver) {
                    Ok(EvaluationResult::Str(result)) => result,
                    Ok(EvaluationResult::Val(json_val)) => json_val.to_string(),
                    _ => template.to_string(), // Fallback on error
                }
            },
            _ => template.to_string(), // Fallback if the initial function creation fails
        }
    }
}
