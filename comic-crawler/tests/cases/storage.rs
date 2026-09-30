use comic_crawler::services::nettruyen::{put_temp_object, rename_to_permanent};

use super::common::ENV_LOCK;

#[tokio::test]
async fn local_storage_writes_temp_and_promotes_file() {
    let _env_lock = ENV_LOCK.lock().unwrap();
    let previous_dir = std::env::var("IMAGE_STORAGE_DIR").ok();
    let storage_dir = std::env::temp_dir().join(format!(
        "axum-replica-storage-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    unsafe {
        std::env::set_var("IMAGE_STORAGE_DIR", &storage_dir);
    }

    let temp_path = put_temp_object("image.jpg", b"image-bytes", "image/jpeg")
        .await
        .unwrap();
    assert_eq!(temp_path, "temp/image.jpg");
    rename_to_permanent(&temp_path, "comic/chapters/1/image.jpg")
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(storage_dir.join("comic/chapters/1/image.jpg")).unwrap(),
        b"image-bytes"
    );
    assert!(!storage_dir.join(&temp_path).exists());

    std::fs::remove_dir_all(&storage_dir).unwrap();
    unsafe {
        if let Some(previous_dir) = previous_dir {
            std::env::set_var("IMAGE_STORAGE_DIR", previous_dir);
        } else {
            std::env::remove_var("IMAGE_STORAGE_DIR");
        }
    }
}
