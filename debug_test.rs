use kotoba_jsonnet::evaluate;

fn main() {
    // Test toLower
    let result = evaluate(r#"std.toLower("HELLO")"#);
    println!("toLower result: {:?}", result);
    
    // Test basic std.length to make sure std is working
    let result = evaluate(r#"std.length("test")"#);
    println!("length result: {:?}", result);
}
