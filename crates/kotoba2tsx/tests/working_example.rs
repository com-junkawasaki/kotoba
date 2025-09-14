//! Working example showing the integrated SWC, CSS, and styled-components functionality

use kotoba2tsx::{
    CssInJsLibrary, KotobaConfig, KotobaComponent, TsxGenerator, ComponentType,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_styled_components_working_example() {
    // Create a generator configured for styled-components
    let generator = TsxGenerator::with_styled_components();

    // Create a simple configuration
    let config = create_working_config();

    // Generate TSX
    let result = generator.generate_tsx(&config);
    assert!(result.is_ok(), "TSX generation should succeed");

    let tsx_code = result.unwrap();

    println!("=== GENERATED TSX WITH STYLED-COMPONENTS ===");
    println!("{}", tsx_code);
    println!("==============================================");

    // Verify the key integrations are working
    assert!(tsx_code.contains("import"), "Should contain imports");
    assert!(tsx_code.contains("styled"), "Should contain styled-components");
    assert!(tsx_code.contains("interface"), "Should contain TypeScript interfaces");
    assert!(tsx_code.contains("const Button"), "Should contain Button component");
    assert!(tsx_code.contains("const App"), "Should contain App component");
    assert!(tsx_code.contains("export default App"), "Should export App");
}

#[test]
fn test_emotion_working_example() {
    // Create a generator configured for Emotion
    let generator = TsxGenerator::with_emotion();

    // Create a simple configuration
    let config = create_working_config();

    // Generate TSX
    let result = generator.generate_tsx(&config);
    assert!(result.is_ok(), "TSX generation should succeed");

    let tsx_code = result.unwrap();

    println!("=== GENERATED TSX WITH EMOTION ===");
    println!("{}", tsx_code);
    println!("===================================");

    // Verify Emotion integration
    assert!(tsx_code.contains("import"), "Should contain imports");
    assert!(tsx_code.contains("emotion"), "Should contain emotion imports");
    assert!(tsx_code.contains("interface"), "Should contain TypeScript interfaces");
    assert!(tsx_code.contains("const Button"), "Should contain Button component");
    assert!(tsx_code.contains("const App"), "Should contain App component");
}

#[test]
fn test_css_processing_demo() {
    use kotoba2tsx::css_processor::CssProcessor;

    let processor = CssProcessor::new();

    // Test CSS with variables
    let css_with_vars = r#"
        --primary-color: #007bff;
        --secondary-color: #6c757d;
        --font-size: 14px;
        --spacing: 8px;
    "#;

    let result = processor.extract_css_variables(css_with_vars, "demo.css");
    assert!(result.is_ok());

    let variables = result.unwrap();
    println!("=== EXTRACTED CSS VARIABLES ===");
    for (name, value) in &variables {
        println!("{}: {}", name, value);
    }
    println!("===============================");

    assert_eq!(variables.get("--primary-color"), Some(&"#007bff".to_string()));
    assert_eq!(variables.get("--font-size"), Some(&"14px".to_string()));

    // Test CSS to JS object conversion
    let simple_css = "color: red; font-size: 16px; background-color: blue;";

    let js_result = processor.css_to_js_object(simple_css, "demo.css");
    assert!(js_result.is_ok());

    let js_object = js_result.unwrap();
    println!("=== CSS TO JS OBJECT ===");
    println!("{}", js_object);
    println!("========================");

    assert!(js_object.contains("color"));
    assert!(js_object.contains("fontSize"));
    assert!(js_object.contains("backgroundColor"));
}

#[test]
fn test_swc_code_generation_demo() {
    use kotoba2tsx::swc_integration::SwcCodeGenerator;

    let swc_gen = SwcCodeGenerator::new();

    // Generate React import
    let react_import = swc_gen.create_react_import(
        vec!["useState".to_string(), "useEffect".to_string(), "useCallback".to_string()],
        Some("React".to_string()),
    );

    println!("=== SWC REACT IMPORT GENERATION ===");
    println!("{}", react_import);
    println!("===================================");

    // Generate TypeScript interface
    let props = vec![
        ("name".to_string(), "string".to_string()),
        ("age".to_string(), "number".to_string()),
        ("disabled".to_string(), "boolean".to_string()),
        ("onClick".to_string(), "() => void".to_string()),
    ];

    let interface = swc_gen.create_props_interface("ButtonProps", props);
    println!("=== SWC TYPESCRIPT INTERFACE ===");
    println!("{}", interface);
    println!("================================");

    // Generate functional component
    let component = swc_gen.create_functional_component("Button", Some("ButtonProps".to_string()));
    println!("=== SWC FUNCTIONAL COMPONENT ===");
    println!("{}", component);
    println!("================================");
}

#[test]
fn test_styled_components_advanced() {
    use kotoba2tsx::styled_components::StyledComponentsGenerator;

    let styled_gen = StyledComponentsGenerator::new();

    // Test styled component with theme interpolation
    let theme_css = r#"
        color: ${props => props.theme.primary || '#007bff'};
        background-color: ${props => props.theme.secondary || '#f8f9fa'};
        font-size: ${props => props.theme.fontSize || '14px'};
        border-radius: 4px;
        padding: 8px 16px;
    "#;

    let result = styled_gen.generate_styled_component_with_theme(
        "ThemedButton",
        theme_css,
        "button",
        vec!["primary".to_string(), "secondary".to_string(), "fontSize".to_string()],
    );

    assert!(result.is_ok());
    let styled_code = result.unwrap();

    println!("=== STYLED COMPONENT WITH THEME ===");
    println!("{}", styled_code);
    println!("====================================");

    assert!(styled_code.contains("const ThemedButton"));
    assert!(styled_code.contains("styled.button"));
    assert!(styled_code.contains("props.theme.primary"));
    assert!(styled_code.contains("props.theme.secondary"));
    assert!(styled_code.contains("props.theme.fontSize"));
}

fn create_working_config() -> KotobaConfig {
    let mut components = HashMap::new();

    // Create a simple Button component with basic styling
    let mut button_props = HashMap::new();
    button_props.insert("className".to_string(), json!("btn"));
    button_props.insert("disabled".to_string(), json!(false));
    // Add some style properties (these will be converted to CSS-in-JS)
    button_props.insert("style_color".to_string(), json!("blue"));
    button_props.insert("style_fontSize".to_string(), json!("16px"));
    button_props.insert("style_padding".to_string(), json!("10px 20px"));

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

    KotobaConfig {
        name: "WorkingDemo".to_string(),
        version: "1.0.0".to_string(),
        theme: "default".to_string(),
        components,
        handlers: HashMap::new(),
        states: HashMap::new(),
        config: HashMap::new(),
    }
}
