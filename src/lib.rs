//! This is a library for converting **structured** XML to JSON based on this ["spec"](https://www.xml.com/pub/a/2006/05/31/converting-between-xml-and-json.html).
//! Uses models from [treexml](https://github.com/rahulg/treexml-rs) and [serde-json](https://github.com/serde-rs/json).
//!
//!
//! ## Example
//! ```
//! let dom_root = treexml::Document::parse("
//!     <population>
//!       <entry>
//!         <name>Alex</name>
//!         <height>173.5</height>
//!       </entry>
//!       <entry>
//!         <name>Mel</name>
//!         <height>180.4</height>
//!       </entry>
//!     </population>
//! ".as_bytes()).unwrap().root.unwrap();
//!
//! assert_eq!(serde_json::Value::Object(node2object::node2object(&dom_root)), serde_json::json!(
//!     {
//!       "population": {
//!         "entry": [
//!           { "name": "Alex", "height": 173.5 },
//!           { "name": "Mel", "height": 180.4 }
//!         ]
//!       }
//!     }
//! ));
//! ```

use serde_json::{Map, Number, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum XMLNodeType {
    Empty,
    Text,
    Attributes,
    TextAndAttributes,
    Parent,
    SemiStructured,
}

fn scan_xml_node(e: &treexml::Element) -> XMLNodeType {
    if e.children.is_empty() {
        if e.text.is_none() && e.cdata.is_none() {
            if e.attributes.is_empty() {
                XMLNodeType::Empty
            } else {
                XMLNodeType::Attributes
            }
        } else if e.attributes.is_empty() {
            XMLNodeType::Text
        } else {
            XMLNodeType::TextAndAttributes
        }
    } else if e.text.is_none() && e.cdata.is_none() {
        XMLNodeType::Parent
    } else {
        XMLNodeType::SemiStructured
    }
}

fn parse_text(text: &str) -> Value {
    if let Ok(v) = text.parse::<f64>() {
        if let Some(v) = Number::from_f64(v) {
            return Value::Number(v);
        }
    }

    if let Ok(v) = text.parse::<bool>() {
        return Value::Bool(v);
    }

    Value::String(text.into())
}

fn parse_text_contents(e: &treexml::Element) -> Value {
    let text = &[&e.text, &e.cdata]
        .iter()
        .map(|v| v.as_ref().map(String::as_str).unwrap_or(""))
        .collect::<Vec<_>>()
        .concat();
    parse_text(&text)
}

fn convert_node_aux(e: &treexml::Element) -> Option<Value> {
    match scan_xml_node(e) {
        XMLNodeType::Parent => {
            let mut data = Map::new();
            let mut firstpass = std::collections::HashSet::<&str>::new();
            let mut vectorized = std::collections::HashSet::<&str>::new();

            if !e.attributes.is_empty() {
                for (k, v) in e.attributes.clone().into_iter() {
                    data.insert(format!("@{}", k), parse_text(&v));
                }
            }

            for c in &e.children {
                if let Some(v) = convert_node_aux(c) {
                    if firstpass.contains(&c.name.as_str()) {
                        if vectorized.contains(&c.name.as_str()) {
                            data.get_mut(&c.name)
                                .unwrap()
                                .as_array_mut()
                                .unwrap()
                                .push(v);
                        } else {
                            let elem = data.remove(&c.name).unwrap();
                            data.insert(c.name.clone(), Value::Array(vec![elem, v]));
                            vectorized.insert(c.name.as_str());
                        }
                    } else {
                        data.insert(c.name.clone(), v);
                        firstpass.insert(c.name.as_str());
                    }
                }
            }
            Some(Value::Object(data))
        }
        XMLNodeType::Text => Some(parse_text_contents(e)),
        XMLNodeType::Attributes => Some(Value::Object(
            e.attributes
                .clone()
                .into_iter()
                .map(|(k, v)| (format!("@{}", k), parse_text(&v)))
                .collect(),
        )),
        XMLNodeType::TextAndAttributes => Some(Value::Object(
            e.attributes
                .clone()
                .into_iter()
                .map(|(k, v)| (format!("@{}", k), parse_text(&v)))
                .chain(vec![("#text".to_string(), parse_text_contents(&e))])
                .collect(),
        )),
        _ => None,
    }
}

/// Converts treexml::Element into a serde_json hashmap. The latter can be wrapped in Value::Object.
pub fn node2object(e: &treexml::Element) -> Map<String, Value> {
    let mut data = Map::new();
    data.insert(e.name.clone(), convert_node_aux(e).unwrap_or(Value::Null));
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn spec_types() {
        for (src, scan_result, conv_result) in vec![
            (r#"<e/>"#, XMLNodeType::Empty, json!({ "e": null })),
            (r#"<e>text</e>"#, XMLNodeType::Text, json!({"e": "text"})),
            (
                r#"<e name="value"/>"#,
                XMLNodeType::Attributes,
                json!({ "e": {"@name": "value"} }),
            ),
            (
                r#"<e name="value">text</e>"#,
                XMLNodeType::TextAndAttributes,
                json!({ "e": { "@name": "value", "#text": "text" } }),
            ),
            (
                r#"<e> <a>text</a> <b>text</b> </e>"#,
                XMLNodeType::Parent,
                json!({ "e": { "a": "text", "b": "text" } }),
            ),
            (
                r#"<e> <a>text</a> <a>text</a> </e>"#,
                XMLNodeType::Parent,
                json!({ "e": { "a": ["text", "text"] } }),
            ),
        ] {
            let fixture = treexml::Document::parse(src.as_bytes())
                .unwrap()
                .root
                .unwrap();

            assert_eq!(scan_result, scan_xml_node(&fixture));
            assert_eq!(conv_result, Value::Object(node2object(&fixture)));
        }
    }

    #[test]
    fn spec_examples() {
        for (src, conv_result) in vec![(
            r#"<e><a>some</a><b>textual</b><a>content</a></e>"#,
            json!({ "e": { "a": [ "some", "content" ], "b": "textual"} }),
        )] {
            let fixture = treexml::Document::parse(src.as_bytes())
                .unwrap()
                .root
                .unwrap();

            assert_eq!(conv_result, Value::Object(node2object(&fixture)));
        }
    }

    #[test]
    fn preserve_attributes_parents() {
        let dom_root = treexml::Document::parse(
            r#"
        <a pizza="hotdog">
          <b frenchfry="milkshake">
            <c>scotch</c>
          </b>
        </a>
    "#
            .as_bytes(),
        )
        .unwrap()
        .root
        .unwrap();

        let json_result = Value::Object(node2object(&dom_root));
        let expected = json!({
            "a": json!({
                "@pizza": "hotdog",
                "b": json!({
                    "@frenchfry": "milkshake",
                    "c":  "scotch"
                })
            })
        });
        assert_eq!(json_result, expected);
    }
}
