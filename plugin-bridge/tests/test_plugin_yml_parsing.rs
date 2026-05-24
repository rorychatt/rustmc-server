use std::io::Write;

use plugin_bridge::java_plugin::JavaPlugin;

fn get_test_dir() -> std::path::PathBuf {
    let test_dir = std::env::current_dir().unwrap().join("target").join("test-tmp");
    std::fs::create_dir_all(&test_dir).unwrap();
    test_dir
}

fn create_test_jar(plugin_yml_content: &str) -> tempfile::NamedTempFile {
    let test_dir = get_test_dir();
    let file = tempfile::Builder::new()
        .suffix(".jar")
        .tempfile_in(&test_dir)
        .expect("Failed to create temp file");

    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file.as_file()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip.start_file("plugin.yml", options)
        .expect("Failed to start plugin.yml");
    zip.write_all(plugin_yml_content.as_bytes())
        .expect("Failed to write plugin.yml");
    zip.finish().expect("Failed to finish zip");

    file
}

#[test]
fn test_parse_valid_plugin_yml() {
    let yml = r#"name: TestPlugin
version: "1.0.0"
main: com.example.TestPlugin
description: "A simple test plugin"
"#;
    let jar = create_test_jar(yml);

    // JavaPlugin::new will fail at JVM init, but we test that the JAR parsing
    // mechanism works correctly by checking the error is NOT about plugin.yml
    let result = JavaPlugin::new_from_jar_meta(jar.path());
    match result {
        Ok(meta) => {
            assert_eq!(meta.name, "TestPlugin");
            assert_eq!(meta.version, "1.0.0");
            assert_eq!(meta.main_class, "com.example.TestPlugin");
            assert_eq!(meta.description, "A simple test plugin");
        }
        Err(e) => panic!("Expected successful parse, got: {e}"),
    }
}

#[test]
fn test_parse_plugin_yml_no_quotes() {
    let yml = "name: SimplePlugin\nversion: 2.0\nmain: org.simple.Main\n";
    let jar = create_test_jar(yml);

    let meta = JavaPlugin::new_from_jar_meta(jar.path()).unwrap();
    assert_eq!(meta.name, "SimplePlugin");
    assert_eq!(meta.version, "2.0");
    assert_eq!(meta.main_class, "org.simple.Main");
}

#[test]
fn test_parse_plugin_yml_single_quotes() {
    let yml = "name: 'QuotedPlugin'\nversion: '3.1.0'\nmain: 'net.test.QuotedPlugin'\ndescription: 'Has single quotes'\n";
    let jar = create_test_jar(yml);

    let meta = JavaPlugin::new_from_jar_meta(jar.path()).unwrap();
    assert_eq!(meta.name, "QuotedPlugin");
    assert_eq!(meta.version, "3.1.0");
    assert_eq!(meta.main_class, "net.test.QuotedPlugin");
}

#[test]
fn test_parse_plugin_yml_missing_main() {
    let yml = "name: NoMain\nversion: 1.0\n";
    let jar = create_test_jar(yml);

    let result = JavaPlugin::new_from_jar_meta(jar.path());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("main"),
        "Error should mention missing 'main' field: {err}"
    );
}

#[test]
fn test_parse_invalid_jar() {
    let test_dir = get_test_dir();
    let file = tempfile::Builder::new()
        .suffix(".jar")
        .tempfile_in(&test_dir)
        .expect("Failed to create temp file");
    std::io::Write::write_all(&mut file.as_file(), b"not a zip file").unwrap();

    let result = JavaPlugin::new_from_jar_meta(file.path());
    assert!(result.is_err());
}

#[test]
fn test_parse_jar_without_plugin_yml() {
    let test_dir = get_test_dir();
    let file = tempfile::Builder::new()
        .suffix(".jar")
        .tempfile_in(&test_dir)
        .expect("Failed to create temp file");

    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file.as_file()));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("SomeClass.class", options).unwrap();
    zip.write_all(b"fake class data").unwrap();
    zip.finish().unwrap();

    let result = JavaPlugin::new_from_jar_meta(file.path());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("plugin.yml"),
        "Error should mention plugin.yml: {err}"
    );
}
