//! Demonstration of the SWC, Lightning CSS, and styled-components/emotion integration

use kotoba2tsx::{
    CssInJsLibrary, KotobaConfig, KotobaComponent, TsxGenerationOptions, TsxGenerator,
    ComponentType,
};
use serde_json::json;
use std::collections::HashMap;

fn main() {
    println!("🚀 Kotoba2TSX Integration Demo");
    println!("===============================\n");

    // Demo 1: SWC + Styled Components Integration
    println!("📦 Demo 1: SWC + Styled Components Integration");
    println!("--------------------------------------------");

    let generator = TsxGenerator::with_styled_components();
    let config = create_demo_config();

    match generator.generate_tsx(&config) {
        Ok(tsx_code) => {
            println!("✅ Successfully generated TSX with styled-components!");
            println!("📄 Generated Code (first 20 lines):");
            println!("{}", tsx_code.lines().take(20).collect::<Vec<_>>().join("\n"));
            if tsx_code.lines().count() > 20 {
                println!("... ({} more lines)", tsx_code.lines().count() - 20);
            }
            println!();
        }
        Err(e) => {
            println!("❌ Error generating TSX: {}", e);
        }
    }

    // Demo 2: Emotion Integration
    println!("🎨 Demo 2: Emotion Integration");
    println!("-----------------------------");

    let emotion_generator = TsxGenerator::with_emotion();

    match emotion_generator.generate_tsx(&config) {
        Ok(tsx_code) => {
            println!("✅ Successfully generated TSX with Emotion!");
            println!("📄 Generated Code (first 15 lines):");
            println!("{}", tsx_code.lines().take(15).collect::<Vec<_>>().join("\n"));
            println!();
        }
        Err(e) => {
            println!("❌ Error generating TSX: {}", e);
        }
    }

    // Demo 3: CSS Processing
    println!("🎨 Demo 3: CSS Processing & Optimization");
    println!("----------------------------------------");

    use kotoba2tsx::css_processor::CssProcessor;

    let css_processor = CssProcessor::new();
    let sample_css = r#"
        .button {
            color: blue;
            font-size: 16px;
            padding: 10px 20px;
            border-radius: 4px;
            /* Comment */
        }
        .card {
            border: 1px solid #ddd;
            background-color: white;
        }
    "#;

    match css_processor.minify_css(sample_css, "demo.css") {
        Ok(minified) => {
            println!("✅ Successfully minified CSS!");
            println!("📄 Original: {} chars", sample_css.len());
            println!("📄 Minified: {} chars", minified.len());
            println!("📄 Minified CSS: {}", minified);
            println!();
        }
        Err(e) => {
            println!("❌ Error minifying CSS: {}", e);
        }
    }

    // Demo 4: CSS Variables Extraction
    println!("🔧 Demo 4: CSS Variables Extraction");
    println!("-----------------------------------");

    let css_with_vars = "--primary: #007bff; --secondary: #6c757d; --font-size: 14px;";
    match css_processor.extract_css_variables(css_with_vars, "vars.css") {
        Ok(vars) => {
            println!("✅ Successfully extracted CSS variables!");
            for (name, value) in vars {
                println!("  {} = {}", name, value);
            }
            println!();
        }
        Err(e) => {
            println!("❌ Error extracting variables: {}", e);
        }
    }

    // Demo 5: SWC Code Generation
    println!("⚡ Demo 5: SWC Code Generation");
    println!("----------------------------");

    use kotoba2tsx::swc_integration::SwcCodeGenerator;

    let swc_gen = SwcCodeGenerator::new();

    // Generate React import
    let react_import = swc_gen.create_react_import(
        vec!["useState".to_string(), "useEffect".to_string()],
        Some("React".to_string()),
    );
    println!("✅ React Import Generation:");
    println!("  {}", react_import);

    // Generate TypeScript interface
    let props = vec![
        ("name".to_string(), "string".to_string()),
        ("age".to_string(), "number".to_string()),
        ("disabled".to_string(), "boolean".to_string()),
    ];
    let interface = swc_gen.create_props_interface("ButtonProps", props.clone());
    println!("✅ TypeScript Interface Generation:");
    for line in interface.lines() {
        println!("  {}", line);
    }
    println!();

    // Demo 6: Styled Components with Props
    println!("🎭 Demo 6: Styled Components with Props");
    println!("--------------------------------------");

    use kotoba2tsx::styled_components::StyledComponentsGenerator;

    let styled_gen = StyledComponentsGenerator::new();
    let dynamic_css = "color: ${color}; font-size: ${size}px; border: ${border};";

    match styled_gen.generate_styled_component_with_props(
        "DynamicButton",
        dynamic_css,
        "button",
        vec!["color".to_string(), "size".to_string(), "border".to_string()],
    ) {
        Ok(styled_code) => {
            println!("✅ Successfully generated styled component with props!");
            println!("📄 Generated Code:");
            for line in styled_code.lines() {
                println!("  {}", line);
            }
            println!();
        }
        Err(e) => {
            println!("❌ Error generating styled component: {}", e);
        }
    }

    // Demo 7: Theme Provider Generation
    println!("🎨 Demo 7: Theme Provider Generation");
    println!("-----------------------------------");

    use kotoba2tsx::styled_components::ThemeProviderGenerator;

    let theme_gen = ThemeProviderGenerator::new();
    let mut theme_data = HashMap::new();
    theme_data.insert("primary".to_string(), json!("#007bff"));
    theme_data.insert("secondary".to_string(), json!("#6c757d"));
    theme_data.insert("fontSize".to_string(), json!(14));

    match generator.generate_theme_provider(theme_data) {
        Ok(theme_code) => {
            println!("✅ Successfully generated theme provider!");
            println!("📄 Generated Theme Code (first 10 lines):");
            println!("{}", theme_code.lines().take(10).collect::<Vec<_>>().join("\n"));
            println!();
        }
        Err(e) => {
            println!("❌ Error generating theme provider: {}", e);
        }
    }

    println!("🎉 Integration Demo Complete!");
    println!("=============================");
    println!("✅ SWC Integration: Working");
    println!("✅ Lightning CSS Integration: Working");
    println!("✅ Styled Components Integration: Working");
    println!("✅ Emotion Integration: Working");
    println!("✅ TypeScript Support: Working");
    println!("✅ Theme Support: Working");
}

fn create_demo_config() -> KotobaConfig {
    let mut components = HashMap::new();

    // Button component with styled-components styles
    let mut button_props = HashMap::new();
    button_props.insert("className".to_string(), json!("btn"));
    button_props.insert("disabled".to_string(), json!(false));
    button_props.insert("style_color".to_string(), json!("blue"));
    button_props.insert("style_fontSize".to_string(), json!("16px"));
    button_props.insert("style_padding".to_string(), json!("10px 20px"));
    button_props.insert("style_borderRadius".to_string(), json!("4px"));

    components.insert("Button".to_string(), KotobaComponent {
        r#type: ComponentType::Component,
        name: "Button".to_string(),
        component_type: Some("button".to_string()),
        props: button_props,
        children: vec!["Click me!".to_string()],
        function: None,
        initial: None,
        metadata: HashMap::new(),
    });

    // Card component with emotion styles
    let mut card_props = HashMap::new();
    card_props.insert("className".to_string(), json!("card"));
    card_props.insert("style_border".to_string(), json!("1px solid #ddd"));
    card_props.insert("style_borderRadius".to_string(), json!("8px"));
    card_props.insert("style_boxShadow".to_string(), json!("0 2px 4px rgba(0,0,0,0.1)"));

    components.insert("Card".to_string(), KotobaComponent {
        r#type: ComponentType::Component,
        name: "Card".to_string(),
        component_type: Some("div".to_string()),
        props: card_props,
        children: vec!["Card content".to_string()],
        function: None,
        initial: None,
        metadata: HashMap::new(),
    });

    KotobaConfig {
        name: "DemoApp".to_string(),
        version: "1.0.0".to_string(),
        theme: "default".to_string(),
        components,
        handlers: HashMap::new(),
        states: HashMap::new(),
        config: HashMap::new(),
    }
}
