use std::fs::{read_to_string, rename, File};
use std::io::{BufWriter, Write};

pub fn load_from_file(filename: &str) -> std::io::Result<Vec<String>> {
    let content = read_to_string(filename)?;
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    if lines.is_empty() {
        Ok(vec![String::new()])
    } else {
        Ok(lines)
    }
}

pub fn save_to_file(lines: &Vec<String>, filename: &str) -> std::io::Result<()> {
    let temp_name = format!("{}.tmp", filename);
    let file = File::create(&temp_name)?;
    let mut writer = BufWriter::new(file);

    for line in lines {
        writer.write_all(line.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;

    rename(temp_name, filename)?;

    Ok(())
}
