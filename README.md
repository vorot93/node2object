# node2object
Convert between XML nodes ([treexml](https://github.com/rahulg/treexml-rs)) and JSON objects ([serde-json](https://github.com/serde-rs/json)).

## Example
```
extern crate treexml;

#[macro_use]
extern crate serde_json;

extern crate node2object;

fn main() {
    let dom_root = treexml::Document::parse("
        <population>
          <entry>
            <name>Alex</name>
            <height>173.5</height>
          </entry>
          <entry>
            <name>Mel</name>
            <height>180.4</height>
          </entry>
        </population>
    ".as_bytes()).unwrap().root.unwrap();
    
    assert_eq!(serde_json::Value::Object(node2object::node2object(&dom_root)), json!(
        {
          "population": {
            "entry": [
              { "name": "Alex", "height": 173.5 },
              { "name": "Mel", "height": 180.4 }
            ]
          }
        }
    )); 
}
```
