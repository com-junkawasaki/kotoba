//! SWC integration for enhanced TypeScript/JavaScript code generation and processing
//!
//! This module provides SWC-based code generation, formatting, and transformation
//! capabilities to improve the quality and performance of generated TSX code.

use crate::error::{Kotoba2TSError, Result};
use swc_core::{
    common::{FileName, SourceMap},
    ecma::{
        ast::*,
        codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter},
        parser::{lexer::Lexer, Capturing, Parser, StringInput, Syntax, TsConfig},
        visit::{VisitMut, VisitMutWith},
    },
};
use std::rc::Rc;

/// SWC-based code generator for enhanced TSX output
pub struct SwcCodeGenerator {
    source_map: Rc<SourceMap>,
    config: CodegenConfig,
}

impl SwcCodeGenerator {
    /// Create a new SWC code generator with default configuration
    pub fn new() -> Self {
        let source_map = Rc::new(SourceMap::default());
        let config = CodegenConfig {
            minify: false,
            ascii_only: false,
            omit_last_semi: false,
            target: swc_core::ecma::codegen::Target::Es2020,
        };

        Self { source_map, config }
    }

    /// Create a new SWC code generator with custom configuration
    pub fn with_config(config: CodegenConfig) -> Self {
        let source_map = Rc::new(SourceMap::default());
        Self { source_map, config }
    }

    /// Format and optimize TypeScript/JavaScript code using SWC
    pub fn format_code(&self, code: &str) -> Result<String> {
        // Parse the code
        let module = self.parse_typescript(code)?;

        // Apply transformations if needed
        let mut module = module;
        // Here we could add various SWC transforms like:
        // - TypeScript stripping
        // - JSX transformation
        // - Minification
        // - etc.

        // Generate formatted code
        self.generate_code(&module)
    }

    /// Parse TypeScript/JSX code into SWC AST
    pub fn parse_typescript(&self, code: &str) -> Result<Module> {
        let file_name = FileName::Anon;
        let source_file = self.source_map.new_source_file(file_name, code.to_string());

        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
                tsx: true,
                decorators: true,
                dts: false,
                no_early_errors: false,
                disallow_ambiguous_jsx_like: false,
            }),
            Default::default(),
            StringInput::from(&*source_file),
            None,
        );

        let capturing = Capturing::new(lexer);
        let mut parser = Parser::new_from(capturing);

        match parser.parse_module() {
            Ok(module) => Ok(module),
            Err(err) => Err(Kotoba2TSError::CodeGeneration(format!(
                "SWC parse error: {}",
                err
            ))),
        }
    }

    /// Generate formatted code from SWC AST
    pub fn generate_code(&self, module: &Module) -> Result<String> {
        let mut buf = vec![];
        {
            let writer = JsWriter::new(self.source_map.clone(), "\n", &mut buf, None);
            let mut emitter = Emitter {
                cfg: self.config.clone(),
                comments: None,
                cm: self.source_map.clone(),
                wr: writer,
            };

            emitter.emit_module(module).map_err(|err| {
                Kotoba2TSError::CodeGeneration(format!("SWC emit error: {}", err))
            })?;
        }

        String::from_utf8(buf).map_err(|err| {
            Kotoba2TSError::CodeGeneration(format!("UTF-8 conversion error: {}", err))
        })
    }

    /// Create a React import declaration
    pub fn create_react_import(&self, items: Vec<String>, default_import: Option<String>) -> ImportDecl {
        let mut specifiers = vec![];

        // Add default import if specified
        if let Some(default) = default_import {
            specifiers.push(ImportSpecifier::Default(ImportDefaultSpecifier {
                span: Default::default(),
                local: Ident::new(default.into(), Default::default()),
            }));
        }

        // Add named imports
        for item in items {
            specifiers.push(ImportSpecifier::Named(ImportNamedSpecifier {
                span: Default::default(),
                local: Ident::new(item.clone().into(), Default::default()),
                imported: Some(ModuleExportName::Ident(Ident::new(
                    item.into(),
                    Default::default(),
                ))),
                is_type_only: false,
            }));
        }

        ImportDecl {
            span: Default::default(),
            specifiers,
            src: Box::new(Str {
                span: Default::default(),
                value: "react".into(),
                raw: Some("\"react\"".into()),
            }),
            type_only: false,
            asserts: None,
        }
    }

    /// Create a styled-components import declaration
    pub fn create_styled_import(&self) -> ImportDecl {
        ImportDecl {
            span: Default::default(),
            specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                span: Default::default(),
                local: Ident::new("styled".into(), Default::default()),
            })],
            src: Box::new(Str {
                span: Default::default(),
                value: "styled-components".into(),
                raw: Some("\"styled-components\"".into()),
            }),
            type_only: false,
            asserts: None,
        }
    }

    /// Create a functional React component
    pub fn create_functional_component(
        &self,
        name: &str,
        props: Vec<Param>,
        body: BlockStmt,
        props_interface: Option<String>,
    ) -> Result<Function> {
        let mut params = vec![];

        // Add props parameter
        if !props.is_empty() {
            params.push(Param {
                span: Default::default(),
                decorators: vec![],
                pat: Pat::Ident(BindingIdent {
                    id: Ident::new("props".into(), Default::default()),
                    type_ann: if props_interface.is_some() {
                        Some(Box::new(TsTypeAnn {
                            span: Default::default(),
                            type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                                span: Default::default(),
                                type_name: TsEntityName::Ident(Ident::new(
                                    format!("{}Props", name).into(),
                                    Default::default(),
                                )),
                                type_params: None,
                            })),
                        }))
                    } else {
                        None
                    },
                }),
            });
        }

        Ok(Function {
            params,
            decorators: vec![],
            span: Default::default(),
            body: Some(body),
            is_generator: false,
            is_async: false,
            type_params: None,
            return_type: Some(Box::new(TsTypeAnn {
                span: Default::default(),
                type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                    span: Default::default(),
                    type_name: TsEntityName::Ident(Ident::new(
                        "JSX.Element".into(),
                        Default::default(),
                    )),
                    type_params: None,
                })),
            })),
        })
    }

    /// Create a TypeScript interface for component props
    pub fn create_props_interface(&self, name: &str, props: Vec<(String, String)>) -> TsInterfaceDecl {
        let mut members = vec![];

        for (prop_name, prop_type) in props {
            members.push(TsTypeElement::TsPropertySignature(TsPropertySignature {
                span: Default::default(),
                readonly: false,
                key: Box::new(Expr::Ident(Ident::new(prop_name.into(), Default::default()))),
                computed: false,
                optional: false,
                type_ann: Some(Box::new(TsTypeAnn {
                    span: Default::default(),
                    type_ann: Box::new(self.parse_type(&prop_type)),
                })),
                init: None,
            }));
        }

        TsInterfaceDecl {
            span: Default::default(),
            id: Ident::new(format!("{}Props", name).into(), Default::default()),
            type_params: None,
            extends: vec![],
            body: TsInterfaceBody {
                span: Default::default(),
                body: members,
            },
            declare: false,
        }
    }

    /// Parse a TypeScript type string into SWC AST
    fn parse_type(&self, type_str: &str) -> TsType {
        // Simple type parsing - in a real implementation, you'd want more comprehensive parsing
        match type_str {
            "string" => TsType::TsKeywordType(TsKeywordType {
                span: Default::default(),
                kind: TsKeywordTypeKind::TsStringKeyword,
            }),
            "number" => TsType::TsKeywordType(TsKeywordType {
                span: Default::default(),
                kind: TsKeywordTypeKind::TsNumberKeyword,
            }),
            "boolean" => TsType::TsKeywordType(TsKeywordType {
                span: Default::default(),
                kind: TsKeywordTypeKind::TsBooleanKeyword,
            }),
            _ => TsType::TsKeywordType(TsKeywordType {
                span: Default::default(),
                kind: TsKeywordTypeKind::TsAnyKeyword,
            }),
        }
    }
}

impl Default for SwcCodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// SWC-based code optimizer
pub struct SwcOptimizer {
    generator: SwcCodeGenerator,
}

impl SwcOptimizer {
    /// Create a new SWC optimizer
    pub fn new() -> Self {
        Self {
            generator: SwcCodeGenerator::new(),
        }
    }

    /// Optimize TypeScript/JavaScript code
    pub fn optimize(&self, code: &str) -> Result<String> {
        // Parse the code
        let mut module = self.generator.parse_typescript(code)?;

        // Apply optimization transforms
        // This is where you could add various SWC optimization passes

        // Generate optimized code
        self.generator.generate_code(&module)
    }

    /// Minify TypeScript/JavaScript code
    pub fn minify(&self, code: &str) -> Result<String> {
        let mut config = CodegenConfig {
            minify: true,
            ascii_only: false,
            omit_last_semi: true,
            target: swc_core::ecma::codegen::Target::Es2020,
        };

        let generator = SwcCodeGenerator::with_config(config);
        let module = generator.parse_typescript(code)?;
        generator.generate_code(&module)
    }
}

impl Default for SwcOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swc_generator_creation() {
        let generator = SwcCodeGenerator::new();
        assert!(!generator.config.minify);
    }

    #[test]
    fn test_create_react_import() {
        let generator = SwcCodeGenerator::new();
        let import = generator.create_react_import(
            vec!["useState".to_string(), "useEffect".to_string()],
            Some("React".to_string()),
        );

        match import.specifiers.first() {
            Some(ImportSpecifier::Default(spec)) => {
                assert_eq!(spec.local.sym, "React");
            }
            _ => panic!("Expected default import"),
        }
    }

    #[test]
    fn test_format_simple_code() {
        let generator = SwcCodeGenerator::new();
        let code = "const x=1;";

        // This should parse and format the code
        let result = generator.parse_typescript(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_props_interface() {
        let generator = SwcCodeGenerator::new();
        let props = vec![
            ("name".to_string(), "string".to_string()),
            ("age".to_string(), "number".to_string()),
        ];

        let interface = generator.create_props_interface("Test", props);
        assert_eq!(interface.id.sym, "TestProps");
    }
}
