//! Convert between XML nodes ([treexml](https://github.com/rahulg/treexml-rs)) and JSON objects ([serde-json](https://github.com/serde-rs/json)).
//!
//! ## Example
//! ```
//! extern crate treexml;
//!
//! #[macro_use]
//! extern crate serde_json;
//!
//! extern crate node2object;
//!
//! fn main() {
//!     let dom_root = treexml::Document::parse("
//!         <population>
//!           <entry>
//!             <name>Alex</name>
//!             <height>173.5</height>
//!           </entry>
//!           <entry>
//!             <name>Mel</name>
//!             <height>180.4</height>
//!           </entry>
//!         </population>
//!     ".as_bytes()).unwrap().root.unwrap();
//!
//!     assert_eq!(serde_json::Value::Object(node2object::node2object(&dom_root)), json!(
//!         {
//!           "population": {
//!             "entry": [
//!               { "name": "Alex", "height": 173.5 },
//!               { "name": "Mel", "height": 180.4 }
//!             ]
//!           }
//!         }
//!     ));
//! }
//! ```

extern crate treexml;

extern crate serde_json;

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
            let mut firstpass = std::collections::HashSet::new();
            let mut vectorized = std::collections::HashSet::new();

            if !e.attributes.is_empty() {
                for (k, v) in e.attributes.clone().into_iter() {
                    data.insert(format!("@{}", k), parse_text(&v));
                }
            }

            for c in &e.children {
                if let Some(v) = convert_node_aux(c) {
                    if firstpass.contains(&c.name) {
                        if vectorized.contains(&c.name) {
                            data.get_mut(&c.name)
                                .unwrap()
                                .as_array_mut()
                                .unwrap()
                                .push(v);
                        } else {
                            let elem = data.remove(&c.name).unwrap();
                            data.insert(c.name.clone(), Value::Array(vec![elem, v]));
                            vectorized.insert(c.name.clone());
                        }
                    } else {
                        data.insert(c.name.clone(), v);
                        firstpass.insert(c.name.clone());
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
    fn node2object_empty() {
        let fixture = treexml::Element::new("e");
        let scan_result = XMLNodeType::Empty;
        let conv_result = json!({ "e": null });

        assert_eq!(scan_result, scan_xml_node(&fixture));
        assert_eq!(conv_result, Value::Object(node2object(&fixture)));
    }

    #[test]
    fn node2object_text() {
        let mut fixture = treexml::Element::new("player");
        fixture.text = Some("Kolya".into());
        let scan_result = XMLNodeType::Text;
        let conv_result = json!({"player": "Kolya"});

        assert_eq!(scan_result, scan_xml_node(&fixture));
        assert_eq!(conv_result, Value::Object(node2object(&fixture)));
    }

    #[test]
    fn node2object_attributes() {
        let mut fixture = treexml::Element::new("player");
        fixture.attributes.insert("score".into(), "9000".into());
        let scan_result = XMLNodeType::Attributes;
        let conv_result = json!({ "player": json!({"@score": 9000.0}) });

        assert_eq!(scan_result, scan_xml_node(&fixture));
        assert_eq!(conv_result, Value::Object(node2object(&fixture)));
    }

    #[test]
    fn node2object_text_and_attributes() {
        let mut fixture = treexml::Element::new("player");
        fixture.text = Some("Kolya".into());
        fixture.attributes.insert("score".into(), "9000".into());
        let scan_result = XMLNodeType::TextAndAttributes;
        let conv_result = json!({ "player": json!({"#text": "Kolya", "@score": 9000.0}) });

        assert_eq!(scan_result, scan_xml_node(&fixture));
        assert_eq!(conv_result, Value::Object(node2object(&fixture)));
    }

    #[test]
    fn node2object_parent() {
        let mut fixture = treexml::Element::new("ServerData");
        fixture.children = vec![
            {
                let mut node = treexml::Element::new("Player");
                node.text = Some("Kolya".into());
                node
            },
            {
                let mut node = treexml::Element::new("Player");
                node.text = Some("Petya".into());
                node
            },
            {
                let mut node = treexml::Element::new("Player");
                node.text = Some("Misha".into());
                node
            },
        ];
        let scan_result = XMLNodeType::Parent;
        let conv_result =
            json!({ "ServerData": json!({ "Player": [ "Kolya", "Petya", "Misha" ] }) });

        assert_eq!(scan_result, scan_xml_node(&fixture));
        assert_eq!(conv_result, Value::Object(node2object(&fixture)));
    }

    #[test]
    fn node2object_preserve_attributes_parents() {
        let dom_root = treexml::Document::parse(
            "
        <a pizza=\"hotdog\">           
          <b frenchfry=\"milkshake\">
            <c>scotch</c>
          </b>
        </a>
    "
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
