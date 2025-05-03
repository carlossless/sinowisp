macro_rules! test_filename {
    ($format:expr) => {{
        let date = chrono::Utc::now().to_rfc3339().replace(":", "-");
        let mut path = std::env::temp_dir();
        path.push(format!(
            "sinowealth-kb-tool-{}-{}.{}",
            date,
            stdext::function_name!()
                .replace(":", "_")
                .replace("__{{closure}}", ""),
            $format
        ));
        path.display().to_string()
    }};
}

pub fn get_fixture_path(filename: &str) -> String {
    let mut path = std::env::current_dir().unwrap();
    path.push("tests");
    path.push("fixtures");
    path.push(filename);
    path.display().to_string()
}
