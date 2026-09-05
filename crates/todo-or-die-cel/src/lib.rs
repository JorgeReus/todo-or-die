use cel_interpreter::{extractors::This, Context, ExecutionError, Program, Value};

fn latest(This(value): This<Value>) -> Result<Value, ExecutionError> {
    Ok(value)
}

pub fn evaluate(source: &str) -> Result<bool, String> {
    evaluate_with_facts(source, serde_json::json!({}))
}

pub fn evaluate_with_facts(source: &str, facts: serde_json::Value) -> Result<bool, String> {
    let program = Program::compile(source).map_err(|e| e.to_string())?;
    let mut context = Context::default();
    context.add_function("latest", latest);
    if let serde_json::Value::Object(values) = facts {
        for (name, value) in values {
            context.add_variable_from_value(
                name,
                cel_interpreter::to_value(value).map_err(|e| e.to_string())?,
            );
        }
    }
    match program.execute(&context).map_err(|e| e.to_string())? {
        Value::Bool(value) => Ok(value),
        value => Err(format!(
            "CEL expression returned {value:?}, expected boolean"
        )),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn evaluates_boolean_escape_hatch() {
        assert!(super::evaluate("true && !false").unwrap());
    }

    #[test]
    fn evaluates_nested_provider_facts() {
        let facts = serde_json::json!({"github": {"issues": {"org/repo#1": {"closed": true}}}});
        assert!(super::evaluate_with_facts("github.issues[\"org/repo#1\"].closed", facts).unwrap());
    }

    #[test]
    fn evaluates_latest_release_method() {
        let facts = serde_json::json!({"github": {"releases": {"org/repo": {"major": 2}}}});
        assert!(super::evaluate_with_facts(
            "github.releases[\"org/repo\"].latest().major >= 2",
            facts
        )
        .unwrap());
    }
}
