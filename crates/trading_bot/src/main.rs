use std::path::PathBuf;

fn main() {
    let mut path = PathBuf::from(r"D:\hello.txt");
    path.push(r"\dir");

    println!("{:?}", path);
}
