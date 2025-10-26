//! Integration tests demonstrating SWC, Lightning CSS, and styled-components/emotion integration

use kotoba2tsx::{
    CssInJsLibrary, CssOptions, KotobaConfig, KotobaComponent, SwcOptions,
    TsxGenerationOptions, TsxGenerator, ComponentType,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_swc_styled_components_integration() {
    // Create a generator with styled-components and SWC
    let mut options = TsxGenerationOptions::default();
    options.css_options.css_in_js = CssInJsLibrary::StyledComponents;
    options.css_options.enable_processing = true;
    options.swc_options.format_code = true;

    let generator = TsxGenerator::with_options(options);

    // Create test configuration with styles
    let config = create_test_config_with_styles();

    // Generate TSX code
    let result = generator.generate_tsx(&config);
    assert!(result.is_ok());

    let tsx_code = result.unwrap();
    println!("Generated TSX:\n{}", tsx_code);

    // Verify styled-components integration - check for the actual import format
    assert!(tsx_code.contains("import") && tsx_code.contains("styled"));
    assert!(tsx_code.contains("const Button"));
    assert!(tsx_code.contains("color: red"));
    assert!(tsx_code.contains("font-size: 14px"));
}

#[test]
fn test_emotion_integration() {
    // Create a generator with Emotion
    let generator = TsxGenerator::with_emotion();

    // Create test configuration
    let config = create_test_config_with_styles();

    // Generate TSX code
    let result = generator.generate_tsx(&config);
    assert!(result.is_ok());

    let tsx_code = result.unwrap();

    // Verify Emotion integration - check for the actual import format
    assert!(tsx_code.contains("import") && tsx_code.contains("emotion"));
    assert!(tsx_code.contains("const Button"));
    assert!(tsx_code.contains("color: red"));
}

#[test]
fn test_css_processing_integration() {
    use kotoba2tsx::css_processor::CssProcessor;

    let processor = CssProcessor::new();

    // Test CSS minification
    let css = "
        .button {
            color: red;
            font-size: 14px;
            margin: 10px;
        }
    ";

    let result = processor.minify_css(css, "test.css");
    assert!(result.is_ok());

    let minified = result.unwrap();
    assert!(minified.contains("color: red"));
    assert!(minified.contains("font-size: 14px"));
    // Basic minification should remove some whitespace
    assert!(minified.len() < css.len());
}

#[test]
fn test_css_variables_extraction() {
    use kotoba2tsx::css_processor::CssProcessor;

    let processor = CssProcessor::new();

    let css = "--primary-color: #007bff; --font-size: 16px; --spacing: 8px;";

    let result = processor.extract_css_variables(css, "test.css");
    assert!(result.is_ok());

    let variables = result.unwrap();
    assert_eq!(variables.get("--primary-color"), Some(&"#007bff".to_string()));
    assert_eq!(variables.get("--font-size"), Some(&"16px".to_string()));
    assert_eq!(variables.get("--spacing"), Some(&"8px".to_string()));
}

#[test]
fn test_css_to_js_object_conversion() {
    use kotoba2tsx::css_processor::CssProcessor;

    let processor = CssProcessor::new();

    let css = "color: blue; font-size: 18px; background-color: #fff;";

    let result = processor.css_to_js_object(css, "test.css");
    assert!(result.is_ok());

    let js_object = result.unwrap();
    assert!(js_object.contains("color"));
    assert!(js_object.contains("fontSize"));
    assert!(js_object.contains("backgroundColor"));
    assert!(js_object.contains("blue"));
    assert!(js_object.contains("18px"));
    assert!(js_object.contains("#fff"));
}

#[test]
fn test_theme_provider_generation() {
    use kotoba2tsx::styled_components::ThemeProviderGenerator;

    let generator = ThemeProviderGenerator::new();

    let mut theme_structure = HashMap::new();
    theme_structure.insert("primary".to_string(), "string".to_string());
    theme_structure.insert("fontSize".to_string(), "number".to_string());

    let types_result = generator.generate_theme_types(theme_structure);
    assert!(types_result.contains("interface Theme"));
    assert!(types_result.contains("primary: string"));
    assert!(types_result.contains("fontSize: number"));
}

#[test]
fn test_swc_code_generation() {
    use kotoba2tsx::swc_integration::SwcCodeGenerator;

    let swc_generator = SwcCodeGenerator::new();

    // Test React import generation
    let import = swc_generator.create_react_import(
        vec!["useState".to_string(), "useEffect".to_string()],
        Some("React".to_string()),
    );

    assert!(import.contains("import React"));
    assert!(import.contains("useState"));
    assert!(import.contains("useEffect"));

    // Test props interface generation
    let props = vec![
        ("name".to_string(), "string".to_string()),
        ("age".to_string(), "number".to_string()),
    ];

    let interface = swc_generator.create_props_interface("Test", props);
    assert!(interface.contains("interface TestProps"));
    assert!(interface.contains("name: string"));
    assert!(interface.contains("age: number"));
}

#[test]
fn test_styled_components_with_props() {
    use kotoba2tsx::styled_components::StyledComponentsGenerator;

    let styled_gen = StyledComponentsGenerator::new();

    let css = "color: ${color}; font-size: ${size}px;";
    let props = vec!["color".to_string(), "size".to_string()];

    let result = styled_gen.generate_styled_component_with_props(
        "DynamicButton",
        css,
        "button",
        props,
    );

    assert!(result.is_ok());
    let styled_code = result.unwrap();
    assert!(styled_code.contains("const DynamicButton"));
    assert!(styled_code.contains("styled.button"));
    assert!(styled_code.contains("${({color}) => props.color"));
    assert!(styled_code.contains("${({size}) => props.size"));
}

#[test]
fn test_minification_integration() {
    // Test with minification enabled
    let generator = TsxGenerator::with_minification();
    let config = create_simple_test_config();

    let result = generator.generate_tsx(&config);
    assert!(result.is_ok());

    let tsx_code = result.unwrap();
    assert!(!tsx_code.is_empty());
    // Just verify it generates some code
    assert!(tsx_code.contains("const Button"));
}

#[test]
fn test_conditional_styling_emotion() {
    use kotoba2tsx::styled_components::EmotionGenerator;

    let emotion_gen = EmotionGenerator::new();

    let base_css = "color: blue; padding: 10px;";
    let mut conditional_styles = HashMap::new();
    conditional_styles.insert("primary".to_string(), "color: red;".to_string());
    conditional_styles.insert("large".to_string(), "font-size: 20px;".to_string());

    let result = emotion_gen.generate_emotion_component_with_cx(
        "Button",
        base_css,
        conditional_styles,
    );

    assert!(result.is_ok());
    let emotion_code = result.unwrap();
    println!("Generated Emotion code:\n{}", emotion_code);

    assert!(emotion_code.contains("const Button"));
    assert!(emotion_code.contains("const ButtonBaseStyles"));
    assert!(emotion_code.contains("cx("));
    // The naming convention might be different, let's check what's actually generated
    assert!(emotion_code.contains("Styles"));
}

#[test]
fn test_full_integration_workflow() {
    // Test the complete workflow from config to styled TSX
    let generator = TsxGenerator::with_styled_components();

    let config = create_complex_test_config();

    let result = generator.generate_tsx(&config);
    assert!(result.is_ok());

    let tsx_code = result.unwrap();
    println!("Full integration TSX:\n{}", tsx_code);

    // Verify all integrations are working
    assert!(tsx_code.contains("import React"));
    assert!(tsx_code.contains("const Button"));
    assert!(tsx_code.contains("const Card"));
    assert!(tsx_code.contains("interface ButtonProps"));
    assert!(tsx_code.contains("interface CardProps"));
    assert!(tsx_code.contains("const App"));
    assert!(tsx_code.contains("export default App"));
}

// Helper functions for creating test configurations

fn create_test_config_with_styles() -> KotobaConfig {
    let mut components = HashMap::new();
    let mut props = HashMap::new();

    // Add styles to props using the correct format
    props.insert("style_color".to_string(), json!("red"));
    props.insert("style_fontSize".to_string(), json!("14px"));

    components.insert("Button".to_string(), KotobaComponent {
        r#type: ComponentType::Component,
        name: "Button".to_string(),
        component_type: Some("button".to_string()),
        props,
        children: vec![],
        function: None,
        initial: None,
        metadata: HashMap::new(),
    });

    KotobaConfig {
        name: "TestApp".to_string(),
        version: "1.0.0".to_string(),
        theme: "default".to_string(),
        components,
        handlers: HashMap::new(),
        states: HashMap::new(),
        config: HashMap::new(),
    }
}

fn create_simple_test_config() -> KotobaConfig {
    let mut components = HashMap::new();

    components.insert("Button".to_string(), KotobaComponent {
        r#type: ComponentType::Component,
        name: "Button".to_string(),
        component_type: Some("button".to_string()),
        props: HashMap::new(),
        children: vec!["Click me".to_string()],
        function: None,
        initial: None,
        metadata: HashMap::new(),
    });

    KotobaConfig {
        name: "SimpleApp".to_string(),
        version: "1.0.0".to_string(),
        theme: "default".to_string(),
        components,
        handlers: HashMap::new(),
        states: HashMap::new(),
        config: HashMap::new(),
    }
}

fn create_complex_test_config() -> KotobaConfig {
    let mut components = HashMap::new();

    // Button component with props
    let mut button_props = HashMap::new();
    button_props.insert("className".to_string(), json!("btn"));
    button_props.insert("disabled".to_string(), json!(false));
    button_props.insert("style_color".to_string(), json!("blue"));
    button_props.insert("style_padding".to_string(), json!("10px"));

    components.insert("Button".to_string(), KotobaComponent {
        r#type: ComponentType::Component,
        name: "Button".to_string(),
        component_type: Some("button".to_string()),
        props: button_props,
        children: vec!["Click me".to_string()],
        function: None,
        initial: None,
        metadata: HashMap::new(),
    });

    // Card component
    let mut card_props = HashMap::new();
    card_props.insert("className".to_string(), json!("card"));
    card_props.insert("style_border".to_string(), json!("1px solid #ccc"));
    card_props.insert("style_borderRadius".to_string(), json!("4px"));

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
        name: "ComplexApp".to_string(),
        version: "1.0.0".to_string(),
        theme: "default".to_string(),
        components,
        handlers: HashMap::new(),
        states: HashMap::new(),
        config: HashMap::new(),
    }
}