use chrono::Duration;
use csv::{Reader, Writer};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Row {
    city: Option<String>,
    country: Option<String>,
    comment: Option<String>,
}

fn main() {
    let v = vec![
        Row {
            city: Some("Kyiv".to_string()),
            country: Some("Ukraine".to_string()),
            comment: Some("comment".to_string()),
        },
        Row {
            city: None,
            country: None,
            comment: None,
        },
        Row {
            city: Some("London".to_string()),
            country: Some("England".to_string()),
            comment: Some("comment".to_string()),
        },
    ];

    let mut w = Writer::from_path(r#"D:\foo.csv"#).unwrap();
    for row in v {
        w.serialize(row).unwrap();
    }

    let mut r = Reader::from_path(r#"D:\foo.csv"#).unwrap();
    for record in r.deserialize() {
        let row: Row = record.unwrap();
        println!("{:?}", row);
    }
}
