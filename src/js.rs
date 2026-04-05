use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(HashMap<String, JsValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsError {
    pub message: String,
}

impl JsError {
    pub fn new(message: impl Into<String>) -> Self {
        JsError { message: message.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomMutation {
    SetAttribute { element_id: String, key: String, value: String },
    SetTextContent { element_id: String, text: String },
    AppendChild { parent_id: String, tag: String, attrs: HashMap<String, String> },
    RemoveElement { element_id: String },
}

pub struct JsRuntime {
    pub globals: HashMap<String, JsValue>,
    pub dom_mutations: Vec<DomMutation>,
    pub console_log: Vec<String>,
}

impl JsRuntime {
    pub fn new() -> Self {
        JsRuntime {
            globals: HashMap::new(),
            dom_mutations: Vec::new(),
            console_log: Vec::new(),
        }
    }

    pub fn execute(&mut self, script: &str) -> Result<JsValue, JsError> {
        let mut last_value = JsValue::Undefined;

        for raw_line in script.lines() {
            let line = raw_line.trim().trim_end_matches(';');
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            last_value = self.eval_statement(line)?;
        }

        Ok(last_value)
    }

    fn eval_statement(&mut self, stmt: &str) -> Result<JsValue, JsError> {
        if let Some(rest) = stmt.strip_prefix("var ").or_else(|| stmt.strip_prefix("let ")).or_else(|| stmt.strip_prefix("const ")) {
            if let Some(eq_pos) = rest.find('=') {
                let name = rest[..eq_pos].trim().to_string();
                let expr = rest[eq_pos + 1..].trim();
                let value = self.eval_expr(expr)?;
                self.globals.insert(name, value.clone());
                return Ok(value);
            }
        }

        if let Some(args_str) = stmt.strip_prefix("console.log(").and_then(|s| s.strip_suffix(')')) {
            let val = self.eval_expr(args_str.trim())?;
            let output = js_value_to_string(&val);
            self.console_log.push(output);
            return Ok(JsValue::Undefined);
        }

        if let Some(dot_pos) = find_method_call(stmt, ".setAttribute(") {
            let var_name = stmt[..dot_pos].trim();
            let args_str = &stmt[dot_pos + ".setAttribute(".len()..];
            if let Some(args_str) = args_str.strip_suffix(')') {
                if let Some((key, value)) = parse_two_string_args(args_str) {
                    let element_id = self.resolve_element_id(var_name);
                    self.dom_mutations.push(DomMutation::SetAttribute {
                        element_id,
                        key,
                        value,
                    });
                    return Ok(JsValue::Undefined);
                }
            }
        }

        if let Some(eq_pos) = stmt.find(".textContent = ") {
            let var_name = stmt[..eq_pos].trim();
            let rhs = stmt[eq_pos + ".textContent = ".len()..].trim();
            let text = parse_string_literal(rhs).unwrap_or_else(|| rhs.to_string());
            let element_id = self.resolve_element_id(var_name);
            self.dom_mutations.push(DomMutation::SetTextContent {
                element_id,
                text,
            });
            return Ok(JsValue::Undefined);
        }

        if let Some(eq_pos) = stmt.find(" = ") {
            let lhs = stmt[..eq_pos].trim();
            if lhs.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let rhs = stmt[eq_pos + 3..].trim();
                let value = self.eval_expr(rhs)?;
                self.globals.insert(lhs.to_string(), value.clone());
                return Ok(value);
            }
        }

        self.eval_expr(stmt)
    }

    fn eval_expr(&mut self, expr: &str) -> Result<JsValue, JsError> {
        let expr = expr.trim();

        if let Ok(n) = expr.parse::<f64>() {
            return Ok(JsValue::Number(n));
        }

        if expr == "true" { return Ok(JsValue::Boolean(true)); }
        if expr == "false" { return Ok(JsValue::Boolean(false)); }
        if expr == "null" { return Ok(JsValue::Null); }
        if expr == "undefined" { return Ok(JsValue::Undefined); }
        if let Some(s) = parse_string_literal(expr) {
            return Ok(JsValue::String(s));
        }

        if let Some(args_str) = expr.strip_prefix("document.getElementById(").and_then(|s| s.strip_suffix(')')) {
            let id = parse_string_literal(args_str.trim()).unwrap_or_else(|| args_str.trim().to_string());
            let mut obj = HashMap::new();
            obj.insert("__element_id__".to_string(), JsValue::String(id));
            return Ok(JsValue::Object(obj));
        }

        if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
            if let Some(val) = self.globals.get(expr) {
                return Ok(val.clone());
            }
        }

        Ok(JsValue::Undefined)
    }

    fn resolve_element_id(&self, var_name: &str) -> String {
        if let Some(val) = self.globals.get(var_name) {
            if let JsValue::Object(map) = val {
                if let Some(JsValue::String(id)) = map.get("__element_id__") {
                    return id.clone();
                }
            }
        }
        var_name.to_string()
    }

    pub fn get_dom_mutations(&self) -> &[DomMutation] {
        &self.dom_mutations
    }

    pub fn clear_mutations(&mut self) {
        self.dom_mutations.clear();
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

fn js_value_to_string(val: &JsValue) -> String {
    match val {
        JsValue::Undefined => "undefined".to_string(),
        JsValue::Null => "null".to_string(),
        JsValue::Boolean(b) => b.to_string(),
        JsValue::Number(n) => {
            if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) }
        }
        JsValue::String(s) => s.clone(),
        JsValue::Object(_) => "[object Object]".to_string(),
    }
}

fn parse_string_literal(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() < 2 {
        return None;
    }
    let first = s.chars().next()?;
    let last = s.chars().last()?;
    if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}

fn find_method_call(stmt: &str, method: &str) -> Option<usize> {
    stmt.find(method)
}

fn parse_two_string_args(args: &str) -> Option<(String, String)> {
    let mut in_quote: Option<char> = None;
    let mut comma_pos = None;
    for (i, c) in args.char_indices() {
        match in_quote {
            Some(q) if c == q => in_quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => in_quote = Some(c),
            None if c == ',' => { comma_pos = Some(i); break; }
            _ => {}
        }
    }
    let comma_pos = comma_pos?;
    let first = parse_string_literal(args[..comma_pos].trim())?;
    let second = parse_string_literal(args[comma_pos + 1..].trim())?;
    Some((first, second))
}

pub fn extract_scripts(html: &str) -> Vec<String> {
    let mut scripts = Vec::new();
    let lower = html.to_lowercase();
    let mut search_from = 0;

    while let Some(open_start) = lower[search_from..].find("<script") {
        let open_start = search_from + open_start;
        if let Some(tag_end) = lower[open_start..].find('>') {
            let content_start = open_start + tag_end + 1;
            if let Some(close_pos) = lower[content_start..].find("</script>") {
                let content = &html[content_start..content_start + close_pos];
                scripts.push(content.to_string());
                search_from = content_start + close_pos + "</script>".len();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    scripts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_runtime_is_empty() {
        let rt = JsRuntime::new();
        assert!(rt.globals.is_empty());
        assert!(rt.dom_mutations.is_empty());
        assert!(rt.console_log.is_empty());
    }

    #[test]
    fn test_execute_var_number() {
        let mut rt = JsRuntime::new();
        rt.execute("var x = 42;").unwrap();
        assert_eq!(rt.globals.get("x"), Some(&JsValue::Number(42.0)));
    }

    #[test]
    fn test_execute_let_string() {
        let mut rt = JsRuntime::new();
        rt.execute(r#"let msg = "hello";"#).unwrap();
        assert_eq!(rt.globals.get("msg"), Some(&JsValue::String("hello".to_string())));
    }

    #[test]
    fn test_execute_const_bool() {
        let mut rt = JsRuntime::new();
        rt.execute("const flag = true;").unwrap();
        assert_eq!(rt.globals.get("flag"), Some(&JsValue::Boolean(true)));
    }

    #[test]
    fn test_console_log_records_output() {
        let mut rt = JsRuntime::new();
        rt.execute(r#"console.log("hello world");"#).unwrap();
        assert_eq!(rt.console_log, vec!["hello world"]);
    }

    #[test]
    fn test_console_log_number() {
        let mut rt = JsRuntime::new();
        rt.execute("console.log(123);").unwrap();
        assert_eq!(rt.console_log, vec!["123"]);
    }

    #[test]
    fn test_get_element_by_id_returns_object() {
        let mut rt = JsRuntime::new();
        let val = rt.execute(r#"document.getElementById("main")"#).unwrap();
        match val {
            JsValue::Object(map) => {
                assert_eq!(map.get("__element_id__"), Some(&JsValue::String("main".to_string())));
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn test_set_attribute_queues_mutation() {
        let mut rt = JsRuntime::new();
        rt.execute(r#"var el = document.getElementById("btn");"#).unwrap();
        rt.execute(r#"el.setAttribute("class", "active");"#).unwrap();
        assert_eq!(rt.dom_mutations.len(), 1);
        assert_eq!(
            rt.dom_mutations[0],
            DomMutation::SetAttribute {
                element_id: "btn".to_string(),
                key: "class".to_string(),
                value: "active".to_string(),
            }
        );
    }

    #[test]
    fn test_set_text_content_queues_mutation() {
        let mut rt = JsRuntime::new();
        rt.execute(r#"var el = document.getElementById("title");"#).unwrap();
        rt.execute(r#"el.textContent = "New Title";"#).unwrap();
        assert_eq!(rt.dom_mutations.len(), 1);
        assert_eq!(
            rt.dom_mutations[0],
            DomMutation::SetTextContent {
                element_id: "title".to_string(),
                text: "New Title".to_string(),
            }
        );
    }

    #[test]
    fn test_get_dom_mutations() {
        let mut rt = JsRuntime::new();
        rt.execute(r#"var el = document.getElementById("x");"#).unwrap();
        rt.execute(r#"el.setAttribute("id", "y");"#).unwrap();
        assert_eq!(rt.get_dom_mutations().len(), 1);
    }

    #[test]
    fn test_clear_mutations() {
        let mut rt = JsRuntime::new();
        rt.execute(r#"var el = document.getElementById("x");"#).unwrap();
        rt.execute(r#"el.setAttribute("id", "y");"#).unwrap();
        rt.clear_mutations();
        assert!(rt.get_dom_mutations().is_empty());
    }

    #[test]
    fn test_unknown_expression_returns_undefined() {
        let mut rt = JsRuntime::new();
        let val = rt.execute("someUnknownCall()").unwrap();
        assert_eq!(val, JsValue::Undefined);
    }

    #[test]
    fn test_execute_does_not_crash_on_arbitrary_input() {
        let mut rt = JsRuntime::new();
        let _ = rt.execute("this is not valid JS at all !!!");
    }

    #[test]
    fn test_extract_scripts_single() {
        let html = r#"<html><head><script>var x = 1;</script></head></html>"#;
        let scripts = extract_scripts(html);
        assert_eq!(scripts, vec!["var x = 1;"]);
    }

    #[test]
    fn test_extract_scripts_multiple() {
        let html = "<script>var a = 1;</script><p>text</p><script>var b = 2;</script>";
        let scripts = extract_scripts(html);
        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts[0], "var a = 1;");
        assert_eq!(scripts[1], "var b = 2;");
    }

    #[test]
    fn test_extract_scripts_none() {
        let html = "<html><body><p>no scripts</p></body></html>";
        let scripts = extract_scripts(html);
        assert!(scripts.is_empty());
    }

    #[test]
    fn test_extract_scripts_with_attributes() {
        let html = r#"<script type="text/javascript">console.log("hi");</script>"#;
        let scripts = extract_scripts(html);
        assert_eq!(scripts, vec![r#"console.log("hi");"#]);
    }

    #[test]
    fn test_multiline_script() {
        let mut rt = JsRuntime::new();
        let script = "var x = 10;\nvar y = 20;\nconsole.log(x);";
        rt.execute(script).unwrap();
        assert_eq!(rt.globals.get("x"), Some(&JsValue::Number(10.0)));
        assert_eq!(rt.globals.get("y"), Some(&JsValue::Number(20.0)));
        assert_eq!(rt.console_log, vec!["10"]);
    }

    #[test]
    fn test_execute_scripts_from_html() {
        let html = r#"<html><body><script>var count = 5;</script></body></html>"#;
        let scripts = extract_scripts(html);
        let mut rt = JsRuntime::new();
        for script in &scripts {
            rt.execute(script).unwrap();
        }
        assert_eq!(rt.globals.get("count"), Some(&JsValue::Number(5.0)));
    }


    #[cfg(test)]
    mod property_tests {
        use super::*;
        use quickcheck::quickcheck;

        #[test]
        fn prop_execute_never_panics() {
            fn check(script: String) -> bool {
                let mut rt = JsRuntime::new();
                let _ = rt.execute(&script);
                true
            }
            quickcheck(check as fn(String) -> bool);
        }

        #[test]
        fn prop_clear_mutations_always_empties() {
            fn check(scripts: Vec<String>) -> bool {
                let mut rt = JsRuntime::new();
                for s in &scripts {
                    let _ = rt.execute(s);
                }
                rt.clear_mutations();
                rt.get_dom_mutations().is_empty()
            }
            quickcheck(check as fn(Vec<String>) -> bool);
        }

        #[test]
        fn prop_var_assignment_stored_in_globals() {
            fn check(name: String, value: f64) -> bool {
                let name: String = name.chars().filter(|c| c.is_ascii_alphabetic()).take(8).collect();
                if name.is_empty() { return true; }
                if matches!(name.as_str(), "var" | "let" | "const" | "true" | "false" | "null") {
                    return true;
                }
                let mut rt = JsRuntime::new();
                let script = format!("var {} = {};", name, value);
                let _ = rt.execute(&script);

                if value.is_finite() {
                    matches!(rt.globals.get(&name), Some(JsValue::Number(_)))
                } else {
                    true 
                }
            }
            quickcheck(check as fn(String, f64) -> bool);
        }
    }
}
