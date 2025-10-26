use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_load_creates_synthia_directory() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    // Change to temp directory
    std::env::set_current_dir(&temp_dir).unwrap();

    // Load should create .synthia/ directory
    let _context = synthia::project_context::ProjectContext::load();

    assert!(temp_dir.path().join(".synthia").exists());

    // Cleanup
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_load_creates_empty_synthia_md() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    let _context = synthia::project_context::ProjectContext::load();

    let synthia_md = temp_dir.path().join(".synthia/.SYNTHIA.md");
    assert!(synthia_md.exists());

    let content = fs::read_to_string(synthia_md).unwrap();
    assert_eq!(content, "");

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_load_reads_existing_content() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    // Create .synthia/.SYNTHIA.md with content
    fs::create_dir_all(temp_dir.path().join(".synthia")).unwrap();
    fs::write(
        temp_dir.path().join(".synthia/.SYNTHIA.md"),
        "Always respond in haiku"
    ).unwrap();

    let context = synthia::project_context::ProjectContext::load();

    assert_eq!(context.custom_instructions, Some("Always respond in haiku".to_string()));

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_empty_file_returns_none() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    let context = synthia::project_context::ProjectContext::load();

    assert_eq!(context.custom_instructions, None);

    std::env::set_current_dir(original_dir).unwrap();
}
