use super::util::ConstOrExpr;
use crate::compiler::prelude::*;

fn make_object_1(values: Vec<Value>) -> Resolved {
    values
        .into_iter()
        .map(make_key_value)
        .collect::<Result<_, _>>()
        .map(Value::Object)
}

fn make_object_2(keys: Vec<Value>, values: Vec<Value>) -> Resolved {
    keys.into_iter()
        .zip(values)
        .map(|(key, value)| Ok((make_key_string(key)?, value)))
        .collect::<Result<_, _>>()
        .map(Value::Object)
}

fn make_key_value(value: Value) -> ExpressionResult<(KeyString, Value)> {
    value.try_array().map_err(Into::into).and_then(|array| {
        let mut iter = array.into_iter();
        let Some(key) = iter.next() else {
            return Err("array value too short".into());
        };
        Ok((make_key_string(key)?, iter.next().unwrap_or(Value::Null)))
    })
}

fn make_key_string(key: Value) -> ExpressionResult<KeyString> {
    match key {
        Value::Bytes(key) => Ok(String::from_utf8_lossy(&key).into()),
        _ => Err("object keys must be strings".into()),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ObjectFromArray;

impl Function for ObjectFromArray {
    fn identifier(&self) -> &'static str {
        "object_from_array"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                keyword: "values",
                kind: kind::ARRAY,
                required: true,
            },
            Parameter {
                keyword: "keys",
                kind: kind::ARRAY,
                required: false,
            },
        ]
    }

    fn examples(&self) -> &'static [Example] {
        &[
            Example {
                title: "create an object from an array of keys/value pairs",
                source: r#"object_from_array([["a", 1], ["b"], ["c", true, 3, 4]])"#,
                result: Ok(r#"{"a": 1, "b": null, "c": true}"#),
            },
            Example {
                title: "create an object from a separate arrays of keys and values",
                source: r#"object_from_array(keys: ["a", "b", "c"], values: [1, null, true])"#,
                result: Ok(r#"{"a": 1, "b": null, "c": true}"#),
            },
        ]
    }

    fn compile(
        &self,
        state: &TypeState,
        _ctx: &mut FunctionCompileContext,
        arguments: ArgumentList,
    ) -> Compiled {
        let values = ConstOrExpr::new(arguments.required("values"), state);
        let keys = arguments
            .optional("keys")
            .map(|keys| ConstOrExpr::new(keys, state));

        Ok(OFAFn { keys, values }.as_expr())
    }
}

#[derive(Clone, Debug)]
struct OFAFn {
    keys: Option<ConstOrExpr>,
    values: ConstOrExpr,
}

impl FunctionExpression for OFAFn {
    fn resolve(&self, ctx: &mut Context) -> Resolved {
        let values = self.values.resolve(ctx)?.try_array()?;
        match &self.keys {
            None => make_object_1(values),
            Some(keys) => make_object_2(keys.resolve(ctx)?.try_array()?, values),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef::object(Collection::any())
    }
}

#[cfg(test)]
mod tests {
    use crate::value;

    use super::*;

    test_function![
        object_from_array => ObjectFromArray;

        makes_object_simple {
            args: func_args![values: value!([["foo", 1], ["bar", 2]])],
            want: Ok(value!({"foo": 1, "bar": 2})),
            tdef: TypeDef::object(Collection::any()),
        }

        uses_keys_parameter {
            args: func_args![keys: value!(["foo", "bar"]), values: value!([1, 2])],
            want: Ok(value!({"foo": 1, "bar": 2})),
            tdef: TypeDef::object(Collection::any()),
        }

        handles_missing_values {
            args: func_args![values: value!([["foo", 1], ["bar"]])],
            want: Ok(value!({"foo": 1, "bar": null})),
            tdef: TypeDef::object(Collection::any()),
        }

        drops_extra_values {
            args: func_args![values: value!([["foo", 1, 2, 3, 4]])],
            want: Ok(value!({"foo": 1})),
            tdef: TypeDef::object(Collection::any()),
        }

        errors_on_missing_keys {
            args: func_args![values: value!([["foo", 1], []])],
            want: Err("array value too short"),
            tdef: TypeDef::object(Collection::any()),
        }
    ];
}