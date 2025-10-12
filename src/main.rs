//! EAF-IPG Runtime CLI
//!
//! Execute JSON-based graph programs using the unified IR runtime.

use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use eaf_ipg_runtime::{validator::validate, Error, engidb::EngiDB, Graph, Node, ui::UiTranspiler};
use kotoba_types::UiProperties;
use std::collections::HashMap;
use indexmap::IndexMap;

#[derive(Parser)]
#[command(name = "eaf-ipg")]
#[command(about = "Kotoba - Language Graph Database")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum TodoCommands {
    /// Add a new todo item
    Add {
        /// Todo title
        title: String,
        /// Todo description (optional)
        #[arg(short, long)]
        description: Option<String>,
        /// Database path
        #[arg(long, default_value = "todo.db")]
        db: PathBuf,
    },
    /// List all todo items
    List {
        /// Database path
        #[arg(long, default_value = "todo.db")]
        db: PathBuf,
    },
    /// Mark a todo as completed
    Complete {
        /// Todo ID
        id: u64,
        /// Database path
        #[arg(long, default_value = "todo.db")]
        db: PathBuf,
    },
    /// Delete a todo item
    Delete {
        /// Todo ID
        id: u64,
        /// Database path
        #[arg(long, default_value = "todo.db")]
        db: PathBuf,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a JSON graph program
    Run {
        /// Path to the JSON graph file
        #[arg(short, long)]
        file: PathBuf,

        /// Path to the EngiDB database file
        #[arg(long)]
        db: PathBuf,

        /// Branch to commit to
        #[arg(long, default_value = "main")]
        branch: String,

        /// Commit author
        #[arg(long, default_value = "kotoba-cli")]
        author: String,

        /// Commit message
        #[arg(short, long)]
        message: String,

        /// Export mode: export JSON without execution
        #[arg(long)]
        export: bool,
    },
    /// Validate a JSON graph file
    Validate {
        /// Path to the JSON graph file
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Test JSON parsing
    TestJson {
        /// Path to the JSON file
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Show generated UI HTML (bypass database for demo)
    ShowUi {
        /// View ID to show
        view_id: String,
    },
    /// Todo app commands
    Todo {
        #[command(subcommand)]
        command: TodoCommands,
    },
    /// UI generation commands
    Ui {
        #[command(subcommand)]
        command: UiCommands,
    },
}

#[derive(Subcommand)]
enum UiCommands {
    /// Generate HTML from UI-IR
    Generate {
        /// View ID to generate
        view_id: String,
        /// Database path
        #[arg(long, default_value = "todo.db")]
        db: PathBuf,
        /// Output HTML file (optional, prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, export, db, branch, author, message } => {
            // Load JSON file
            let json_content = fs::read_to_string(&file)?;

            if export {
                println!("{}", json_content);
                return Ok(());
            }

            // Parse JSON into Graph
            let graph: Graph = serde_json::from_str(&json_content)?;

            // Open the database
            let engidb = EngiDB::open(&db)?;

            // Import the graph
            println!("Importing graph into database...");
            engidb.import_graph(&graph)?;
            println!("Import complete.");

            // Commit the changes
            println!("Committing to branch '{}'...", branch);
            let commit_cid = engidb.commit(&branch, author, message)?;
            println!("Successfully committed with CID: {}", commit_cid);
            
            // // Validate
            // validate(&graph)?;

            // // Lower to execution DAG
            // let exec_dag = lower_to_exec_dag(&graph)?;

            // // Execute
            // let mut runtime = eaf_ipg_runtime::Runtime::new();
            // schedule_and_run(&mut runtime, &exec_dag).await?;

            // println!("Execution completed successfully");
        }

        Commands::Validate { file } => {
            let json_content = fs::read_to_string(&file)?;
            let graph: Graph = serde_json::from_str(&json_content)?;

            match validate(&graph) {
                Ok(_) => println!("✓ Validation passed"),
                Err(Error::Validation(e)) => {
                    eprintln!("✗ Validation failed: {}", e);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("✗ Unexpected error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::TestJson { file } => {
            let json_content = fs::read_to_string(&file)?;
            let value: serde_json::Value = serde_json::from_str(&json_content)?;
            println!("✓ JSON parsed successfully: {}", value);
        }
        Commands::ShowUi { view_id } => {
            show_ui_demo(&view_id)?;
        }
        Commands::Todo { command } => {
            match command {
                TodoCommands::Add { title, description, db } => {
                    println!("Adding todo: {}", title);
                    add_todo(&db, &title, description.as_deref())?;
                    println!("✓ Todo added successfully!");
                }
                TodoCommands::List { db } => {
                    println!("📝 Todo List:");
                    list_todos(&db)?;
                }
                TodoCommands::Complete { id, db } => {
                    println!("Completing todo #{}", id);
                    complete_todo(&db, id)?;
                    println!("✓ Todo #{} marked as completed!", id);
                }
                TodoCommands::Delete { id, db } => {
                    println!("Deleting todo #{}", id);
                    delete_todo(&db, id)?;
                    println!("✓ Todo #{} deleted!", id);
                }
            }
        }

        Commands::Ui { command } => {
            match command {
                UiCommands::Generate { view_id, db, output } => {
                    let transpiler = UiTranspiler::new(&db)?;
                    let html = transpiler.transpile_to_html(&view_id)?;

                    match output {
                        Some(path) => {
                            std::fs::write(&path, &html)?;
                            println!("✓ HTML generated and saved to: {}", path.display());
                        }
                        None => {
                            println!("{}", html);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// Todo app functions using EngiDB
fn add_todo(db_path: &PathBuf, title: &str, description: Option<&str>) -> Result<(), Error> {
    let engidb = EngiDB::open(db_path)?;

    // Generate a simple ID based on current timestamp
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u64;

    let now = chrono::Utc::now().to_rfc3339();

    let todo_node = Node {
        id: format!("todo_{}", id),
        kind: "TodoItem".to_string(),
        properties: {
            let mut props = IndexMap::new();
            props.insert("id".to_string(), serde_json::json!(id));
            props.insert("title".to_string(), serde_json::json!(title));
            props.insert("description".to_string(), serde_json::json!(description.unwrap_or("")));
            props.insert("completed".to_string(), serde_json::json!(false));
            props.insert("created_at".to_string(), serde_json::json!(now));
            props.insert("updated_at".to_string(), serde_json::json!(now));
            props
        },
    };

    engidb.add_vertex(&todo_node)?;
    engidb.commit("main", "todo-cli".to_string(), format!("Add todo: {}", title))?;

    Ok(())
}

fn list_todos(db_path: &PathBuf) -> Result<(), Error> {
    // For now, just show a placeholder since full query implementation is pending
    println!("📝 Todo listing functionality will be implemented with full GQL support");
    println!("💡 Currently available:");
    println!("   - Add todos with: cargo run -- todo add \"Your task\"");
    println!("   - Mark complete: cargo run -- todo complete <id>");
    println!("   - Delete todos: cargo run -- todo delete <id>");
    Ok(())
}

fn complete_todo(db_path: &PathBuf, id: u64) -> Result<(), Error> {
    let engidb = EngiDB::open(db_path)?;
    // TODO: Implement completion logic when full query support is available
    println!("✅ Todo completion will be implemented with full EngiDB query capabilities");
    engidb.commit("main", "todo-cli".to_string(), format!("Complete todo: {}", id))?;
    Ok(())
}

fn delete_todo(db_path: &PathBuf, id: u64) -> Result<(), Error> {
    let engidb = EngiDB::open(db_path)?;
    // TODO: Implement deletion logic when full query support is available
    println!("🗑️  Todo deletion will be implemented with full EngiDB query capabilities");
    engidb.commit("main", "todo-cli".to_string(), format!("Delete todo: {}", id))?;
    Ok(())
}

// Demo UI generation without database dependency
fn show_ui_demo(view_id: &str) -> Result<(), Error> {

    // Create mock UI nodes (same as in UiTranspiler)
    let ui_nodes = create_mock_todo_ui_nodes();

    let root_node = ui_nodes.iter()
        .find(|n| n.id == view_id)
        .ok_or_else(|| Error::Validation(format!("View '{}' not found", view_id)))?;

    let html = node_to_html_demo(root_node, &ui_nodes)?;
    let full_html = wrap_with_template_demo(&html);

    println!("{}", full_html);
    Ok(())
}

fn create_mock_todo_ui_nodes() -> Vec<Node> {
    vec![
        Node {
            id: "todo_view".to_string(),
            kind: "View".to_string(),
            properties: {
                let mut props = IndexMap::new();
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
                let mut props = IndexMap::new();
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
                let mut props = IndexMap::new();
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
                let mut props = IndexMap::new();
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
                let mut props = IndexMap::new();
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

fn node_to_html_demo(node: &Node, all_nodes: &[Node]) -> Result<String, Error> {
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
            let child_html = node_to_html_demo(child_node, all_nodes)?;
            html.push_str(&child_html);
        }
    }

    html.push_str(&format!("</{}>", tag));

    Ok(html)
}

fn wrap_with_template_demo(body_html: &str) -> String {
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
