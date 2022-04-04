use csv::Reader;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Record {
    setting_name: String,
    setting_value: String,
}

fn main() {
    let mut reader =
        Reader::from_path(r"D:\work_and_projects\Goshik_bot\rust_bot_DO\step_settings.csv")
            .unwrap();

    for record in reader.deserialize::<Record>() {
        println!("{:?}", record.unwrap())
    }
}
