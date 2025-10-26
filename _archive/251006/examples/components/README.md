# Component Examples

This directory contains examples of component definitions and handlers in Kotoba.

## Examples

### Basic Components
- `minimal-kotoba.kotoba` - Minimal example showing how to define components in Jsonnet for TSX conversion

### Advanced Handlers
- `advanced-handlers.kotoba` - Examples of advanced HTTP handlers and component patterns

## Component System

Kotoba uses a component-based architecture where you define UI components using Jsonnet objects. These components can be:

- Converted to React/TSX components
- Used in server-side rendering
- Combined with built-in HTTP handlers

## Running Examples

To convert components to TSX:

```bash
cd examples/components
kotoba build minimal-kotoba.kotoba --target tsx
```

To run as a web server:

```bash
kotoba serve advanced-handlers.kotoba
```
